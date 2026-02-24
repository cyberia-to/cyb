import { createAsyncThunk, createSelector, createSlice, PayloadAction } from '@reduxjs/toolkit';
import { isWorker } from 'src/constants/config';
import { SenseApi } from 'src/contexts/backend/services/senseApi';
import { RootState } from 'src/redux/store';
import { MsgMultiSendValue, MsgSendValue } from 'src/services/backend/services/indexer/types';
import {
  SenseItemLinkMeta,
  SenseListItem,
  SenseListItemTransactionMeta,
} from 'src/services/backend/types/sense';
import { EntryType } from 'src/services/CozoDb/types/entities';
import { isParticle } from '../../particle/utils';
// Add this import for generating unique thread IDs
import { SenseItemId } from '../types/sense';

// similar to blockchain/tx/message type
export type SenseItem = {
  id: SenseItemId;
  transactionHash: string;

  // add normal type
  type: string;

  meta: SenseListItem['meta'];
  timestamp: string;
  memo: string | undefined;
  from: string;

  // for optimistic update
  status?: 'pending' | 'error';
  fromLog?: boolean;
};

type Chat = {
  id: SenseItemId;
  isLoading: boolean;
  error: string | undefined;
  data: SenseItem[];
  unreadCount: number;
};

type SliceState = {
  list: {
    isLoading: boolean;
    data: string[];
    error: string | undefined;
  };
  chats: {
    [key in SenseItemId]?: Chat;
  };
  summary: {
    unreadCount: {
      total: number;
      particles: number;
      neurons: number;
    };
  };
  llm: {
    // Change from messages array to threads array
    threads: LLMThread[];
    currentThreadId: string | null; // Keep track of the currently selected thread
  };
};

const initialState: SliceState = {
  list: {
    isLoading: false,
    data: [],
    error: undefined,
  },
  chats: {},
  summary: {
    unreadCount: {
      total: 0,
      particles: 0,
      neurons: 0,
    },
  },
  llm: {
    // Change from messages array to threads array
    threads: !isWorker()
      ? (JSON.parse(localStorage.getItem('llmThreads') || '[]') as LLMThread[])
      : [],
    currentThreadId: null, // Keep track of the currently selected thread
  },
};

function formatApiData(item: SenseListItem): SenseItem {
  if (item.entryType === EntryType.chat && item.meta.to) {
    item.entryType = EntryType.particle;
  }

  const { meta } = item;

  const formatted: SenseItem = {
    timestamp: new Date(meta.timestamp).toISOString(),

    // lol
    transactionHash:
      item.transactionHash ||
      item.hash ||
      item.meta.transaction_hash ||
      item.meta.hash ||
      item.meta.transactionHash,

    memo: item.memo || meta.memo,

    senseChatId: item.id,
    // not good
    unreadCount: item.unreadCount || 0,
  };

  switch (item.entryType) {
    case EntryType.chat:
    case EntryType.transactions: {
      const meta = item.meta as SenseListItemTransactionMeta;
      const { type } = meta;

      let from = item.ownerId;

      if (type === 'cosmos.bank.v1beta1.MsgSend') {
        const value = meta.value as MsgSendValue;
        from = value.fromAddress;
      } else if (type === 'cosmos.bank.v1beta1.MsgMultiSend') {
        const value = meta.value as MsgMultiSendValue;

        from = value.inputs[0].address;
      }

      Object.assign(formatted, {
        type,
        from,
        meta: item.meta.value,
      });

      break;
    }

    case EntryType.particle: {
      const meta = item.meta as SenseItemLinkMeta;

      Object.assign(formatted, {
        type: 'cyber.graph.v1beta1.MsgCyberlink',
        from: meta.neuron,
        meta,
        fromLog: true,
      });

      break;
    }

    default:
      return {};
  }

  return formatted;
}

const getSenseList = createAsyncThunk('sense/getSenseList', async (senseApi: SenseApi) => {
  const data = await senseApi!.getList();
  return data.map(formatApiData);
});

const getSenseChat = createAsyncThunk(
  'sense/getSenseChat',
  async ({ id, senseApi }: { id: SenseItemId; senseApi: SenseApi }) => {
    const particle = isParticle(id);

    if (particle) {
      const links = await senseApi!.getLinks(id);
      const formattedLinks = links.map((item) => {
        if (item.timestamp === 0) {
          // FIXME:
          return;
        }
        return formatApiData({
          ...item,
          id,
          entryType: EntryType.particle,
          meta: item,
        });
      });

      return formattedLinks.filter(Boolean);
    }

    const data = await senseApi!.getFriendItems(id);
    const formattedData = data.map((item) => {
      const entryType = item.to ? EntryType.particle : EntryType.chat;
      return formatApiData({
        ...item,
        entryType,
        id,
        meta: item,
      });
    });

    return formattedData;
  }
);

const markAsRead = createAsyncThunk(
  'sense/markAsRead',
  async ({ id, senseApi }: { id: SenseItemId; senseApi: SenseApi }) => {
    return senseApi!.markAsRead(id);
  }
);

const newChatStructure: Chat = {
  id: '',
  isLoading: false,
  data: [],
  error: undefined,
  unreadCount: 0,
};

function checkIfMessageExists(chat: Chat, newMessage: SenseItem) {
  const lastMessages = chat.data.slice(-5);

  const isMessageExists = lastMessages.some((msg) => {
    return msg.transactionHash === newMessage.transactionHash;
  });

  return isMessageExists;
}

// Add LLM types
export interface LLMMessage {
  text: string;
  sender: 'user' | 'llm';
  timestamp: number;
}

export interface LLMThread {
  id: string;
  title?: string;
  dateUpdated: number;
  messages: LLMMessage[];
}

const slice = createSlice({
  name: 'sense',
  initialState,
  reducers: {
    // backend may push this action
    updateSenseList: {
      reducer: (state, action: PayloadAction<SenseItem[]>) => {
        const data = action.payload;

        data.forEach((message) => {
          const { senseChatId: id } = message;

          if (!state.chats[id]) {
            state.chats[id] = { ...newChatStructure };
          }

          const chat = state.chats[id]!;

          Object.assign(chat, {
            id,
            // fix ts
            unreadCount: message.unreadCount || 0,
          });

          if (!checkIfMessageExists(chat, message)) {
            chat.data = chat.data.concat(message);
          }
        });

        slice.caseReducers.orderSenseList(state);
      },
      prepare: (data: SenseListItem[]) => {
        return {
          payload: data.map(formatApiData),
        };
      },
    },
    // optimistic update
    addSenseItem(state, action: PayloadAction<{ id: SenseItemId; item: SenseItem }>) {
      const { id, item } = action.payload;
      const chat = state.chats[id]!;

      chat.data.push({
        ...item,
        meta: item.meta,
        status: 'pending',
      });

      const newList = state.list.data.filter((item) => item !== id);
      newList.unshift(id);
      state.list.data = newList;
    },
    // optimistic confirm/error
    updateSenseItem(
      state,
      action: PayloadAction<{
        chatId: SenseItemId;
        txHash: string;
        isSuccess: boolean;
      }>
    ) {
      const { chatId, txHash, isSuccess } = action.payload;
      const chat = state.chats[chatId]!;

      const item = chat.data.find((item) => item.transactionHash === txHash);

      if (item) {
        if (isSuccess) {
          delete item.status;
        } else {
          item.status = 'error';
        }
      }
    },
    orderSenseList(state) {
      const chatsLastMessage = Object.keys(state.chats).reduce<
        {
          id: string;
          lastMsg: SenseItem;
        }[]
      >((acc, id) => {
        const chat = state.chats[id]!;

        // may be loading this moment, no data
        if (!chat.data.length) {
          return acc;
        }

        const lastMsg = chat.data[chat.data.length - 1];
        acc.push({ id, lastMsg });

        return acc;
      }, []);

      const sorted = chatsLastMessage.sort((a, b) => {
        return Date.parse(b.lastMsg.timestamp) - Date.parse(a.lastMsg.timestamp);
      });

      state.list.data = sorted.map((i) => i.id);
    },
    reset() {
      return initialState;
    },
    // LLM reducers
    createLLMThread(state, action: PayloadAction<{ id: string; title?: string }>) {
      const newThread: LLMThread = {
        id: action.payload.id,
        messages: [],
        dateUpdated: Date.now(),
        title: action.payload.title || `Conversation ${state.llm.threads.length + 1}`,
      };
      state.llm.threads.push(newThread);
      state.llm.currentThreadId = action.payload.id;
      localStorage.setItem('llmThreads', JSON.stringify(state.llm.threads));
    },

    selectLLMThread(state, action: PayloadAction<{ id: string }>) {
      state.llm.currentThreadId = action.payload.id;
    },

    addLLMMessageToThread(state, action: PayloadAction<{ threadId: string; message: LLMMessage }>) {
      const thread = state.llm.threads.find((t) => t.id === action.payload.threadId);
      if (thread) {
        thread.messages.push(action.payload.message);
        thread.dateUpdated = action.payload.message.timestamp;
        localStorage.setItem('llmThreads', JSON.stringify(state.llm.threads));
      }
    },

    // Add action to replace the last message (for updating "waiting..." message)
    replaceLastLLMMessageInThread(
      state,
      action: PayloadAction<{ threadId: string; message: LLMMessage }>
    ) {
      const thread = state.llm.threads.find((t) => t.id === action.payload.threadId);
      if (thread && thread.messages.length > 0) {
        thread.messages[thread.messages.length - 1] = action.payload.message;
        localStorage.setItem('llmThreads', JSON.stringify(state.llm.threads));
      }
    },

    deleteLLMThread(state, action: PayloadAction<{ id: string }>) {
      const newT = state.llm.threads.filter((thread) => thread.id !== action.payload.id);

      console.log('newT', newT);

      state.llm.threads = newT;

      if (state.llm.currentThreadId === action.payload.id) {
        state.llm.currentThreadId = null;
      }

      // Object.assign(state.llm, {
      //   threads: newT,
      // });

      localStorage.setItem('llmThreads', JSON.stringify(state.llm.threads));
    },

    clearLLMThreads(state) {
      state.llm.threads = [];
      state.llm.currentThreadId = null;
      localStorage.removeItem('llmThreads');
    },
  },

  extraReducers: (builder) => {
    builder.addCase(getSenseList.pending, (state) => {
      state.list.isLoading = true;
    });

    builder.addCase(getSenseList.fulfilled, (state, action) => {
      state.list.isLoading = false;

      const newList: SliceState['list']['data'] = [];

      action.payload.forEach((message) => {
        const { senseChatId: id } = message;

        if (!state.chats[id]) {
          state.chats[id] = { ...newChatStructure };
        }

        const chat = state.chats[id]!;

        Object.assign(chat, {
          id,
          // fix
          unreadCount: message.unreadCount || 0,
        });

        if (!checkIfMessageExists(chat, message)) {
          chat.data = chat.data.concat(message);
        }

        newList.push(id);
      });

      state.list.data = newList;
    });
    builder.addCase(getSenseList.rejected, (state, action) => {
      console.error(action);

      state.list.isLoading = false;
      state.list.error = action.error.message;
    });

    builder.addCase(getSenseChat.pending, (state, action) => {
      const { id } = action.meta.arg;

      if (!state.chats[id]) {
        state.chats[id] = { ...newChatStructure };
      }

      // don't understand why ts warning
      state.chats[id].isLoading = true;
    });

    builder.addCase(getSenseChat.fulfilled, (state, action) => {
      const { id } = action.meta.arg;
      const chat = state.chats[id]!;
      chat.isLoading = false;

      chat.id = id;

      chat.data = action.payload;
    });
    builder.addCase(getSenseChat.rejected, (state, action) => {
      console.error(action);

      const chat = state.chats[action.meta.arg.id]!;
      chat.isLoading = false;
      chat.error = action.error.message;
    });

    // maybe add .pending, .rejected
    // can be optimistic
    builder.addCase(markAsRead.fulfilled, (state, action) => {
      const { id } = action.meta.arg;
      const chat = state.chats[id]!;

      const particle = isParticle(id);

      const { unreadCount } = chat;

      state.summary.unreadCount.total -= unreadCount;
      if (particle) {
        state.summary.unreadCount.particles -= unreadCount;
      } else {
        state.summary.unreadCount.neurons -= unreadCount;
      }

      chat.unreadCount = 0;
    });
  },
});

const selectUnreadCounts = createSelector(
  (state: RootState) => state.sense.chats,
  (chats) => {
    let unreadCountParticle = 0;
    let unreadCountNeuron = 0;

    Object.values(chats).forEach(({ id, unreadCount }) => {
      const particle = isParticle(id);

      if (particle) {
        unreadCountParticle += unreadCount;
      } else {
        unreadCountNeuron += unreadCount;
      }
    });

    const total = unreadCountParticle + unreadCountNeuron;

    return {
      total,
      particles: unreadCountParticle,
      neurons: unreadCountNeuron,
    };
  }
);

export const {
  addSenseItem,
  updateSenseItem,
  updateSenseList,
  reset,
  createLLMThread,
  deleteLLMThread,
  selectLLMThread,
  addLLMMessageToThread,
  replaceLastLLMMessageInThread,
  clearLLMThreads,
} = slice.actions;

export { getSenseChat, getSenseList, markAsRead };

// selectors
export { selectUnreadCounts };

export default slice.reducer;
