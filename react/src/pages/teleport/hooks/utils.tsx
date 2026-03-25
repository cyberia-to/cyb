import { toString as uint8ArrayToAsciiString } from 'uint8arrays/to-string';

export const parseEventsEndBlockEvents = (events: readonly Event[]) => {
  try {
    const data = [];
    // eslint-disable-next-line no-restricted-syntax
    for (const event of events) {
      if (event.type === 'swap_transacted') {
        const { attributes } = event;

        // attributes.map((item) =>
        //   console.log(
        //     uint8ArrayToAsciiString(item.key),
        //     uint8ArrayToAsciiString(item.value)
        //   )
        // );

        const exchangedDemandCoinAmountAttr = attributes.find(
          (attr) => uint8ArrayToAsciiString(attr.key) === 'exchanged_demand_coin_amount'
        );

        const exchangedOfferCoinAmountAttr = attributes.find(
          (attr) => uint8ArrayToAsciiString(attr.key) === 'exchanged_offer_coin_amount'
        );

        const exchangedDemandCoinAmountValue = exchangedDemandCoinAmountAttr
          ? uint8ArrayToAsciiString(exchangedDemandCoinAmountAttr.value)
          : undefined;

        const exchangedOfferCoinAmountValue = exchangedOfferCoinAmountAttr
          ? uint8ArrayToAsciiString(exchangedOfferCoinAmountAttr.value)
          : undefined;

        const msgIndexAttr = attributes.find(
          (attr) => uint8ArrayToAsciiString(attr.key) === 'msg_index'
        );

        const successAttr = attributes.find(
          (attr) => uint8ArrayToAsciiString(attr.key) === 'success'
        );

        const successValue = successAttr ? uint8ArrayToAsciiString(successAttr.value) : undefined;

        const msgIndexValue = msgIndexAttr
          ? uint8ArrayToAsciiString(msgIndexAttr.value)
          : undefined;

        data.push({
          msgIndex: msgIndexValue,
          exchangedDemandCoinAmount: exchangedDemandCoinAmountValue,
          exchangedOfferCoinAmount: exchangedOfferCoinAmountValue,
          success: successValue,
        });
      }
    }
    return data;
  } catch (_error) {
    return undefined;
  }
};

export const parseEventsTxsSwap = (log: Log[]) => {
  try {
    if (log && Object.keys(log).length) {
      const [{ events }] = log;

      if (events) {
        // eslint-disable-next-line no-restricted-syntax
        for (const event of events) {
          if (event.type === 'swap_within_batch') {
            const { attributes } = event;

            // REFACTOR: CREATE HELPER <<<<
            const demandCoinDenomAttr = attributes.find((attr) => attr.key === 'demand_coin_denom');
            const demandCoinDenomValue = demandCoinDenomAttr
              ? demandCoinDenomAttr.value
              : undefined;

            // >>>>> REFACTOR: CREATE HELPER

            const offerCoinDenomAttr = attributes.find((attr) => attr.key === 'offer_coin_denom');
            const offerCoinDenomValue = offerCoinDenomAttr ? offerCoinDenomAttr.value : undefined;

            const msgIndexAttr = attributes.find((attr) => attr.key === 'msg_index');
            const msgIndexValue = msgIndexAttr ? msgIndexAttr.value : undefined;

            if (demandCoinDenomValue && msgIndexValue && offerCoinDenomValue) {
              return {
                msgIndex: msgIndexValue,
                demandCoinDenom: demandCoinDenomValue,
                offerCoinDenom: offerCoinDenomValue,
              };
            }
          }
        }
      }
    }
    return undefined;
  } catch (_error) {
    return undefined;
  }
};
