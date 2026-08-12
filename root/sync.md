## General

- every process is RxJs async loop with N-second interval
- all CID from's from any links in any process goes to separate _particle resolver queue(non-blocking)_

## My transactions process

Sync all current neuron transactions: extract tweets, and chat(any interractions with other neurons).

_transactions can be heavy, for ex. there can be MultiSend/broadcast transactions of 150-200Kb_

- save all transactions to DB
- create sense chats for all unique neurons from MsgSend/MsgMultisend
- extract 'tweet' links and add it to sync items(SyncParticlesLoop)
- 
> **request count** = 1 aggregate req. + [user_transactions_count/batch].
> example - 1000 trans., batch is 200, requests = 1 + 1000/200 = 6
> every 60 sec.



```mermaid
flowchart TD
    A[SyncTransactionLoop] --> P1
    P1[get last timestamp] --> B
    B(get trans. count > timestamp) <-..->|messages_by_address_aggregate| I[(Indexer)]
    B --> C{has new?}
    C -->|yes| D(fetch batch transactions)
    D <-..->|messages_by_address 'batch=200'| I
    D -->|iter by batch| IP{batch processor}
    IP --> D
    IP -->|has items| E{transaction type}
    IP -->|complete| U(update last timestamp)
    E -->|MsgSend/MsgMultiSend| T1(add neuron to sense chat)
    E -->|Cyberlink 'tweet'| T2(add tweet to 'sync')
```

## My particles(tweets) process
Sync current user particle updates(new links)

> **request count** = user_links_from='tweet' \* [links_count/batch]
> example - 100 tweets. requests = \* [???/10] = [100...???]
> every 60 sec.

```mermaid
flowchart TD
    A[SyncParticlesLoop] -->|interval| P1
    P1[get user tweets with last timestamp] --> L
    L[tweet processor] -->|iter by every tweet| F
    F[fetch links > timestamp] --> L
    F <-..->|Cyberlinks| I[(Indexer)]
```

## My friends process
Sync current user updates from friends(tweets&follows)

> **request count** = following*users * (1(indexer) + [2(lcd)])
> example - following is 10, requests = 10 \* (1 + [2]) = [10...30]
> every 60 sec.

```mermaid
flowchart TD
    A[SyncMyFriends] -->|interval| L
    L[my friends processor]<-->|iter by every friend| F
    F(fetch count)<--> L
    F <-..->|cyberlinks_aggregate 'tweets', 'follow'| I[(Indexer)]
    F --> C{has new follows/tweets?}
    C -->|yes| D(fetch links)
    D <-..->|fetch 'tweets' > timestamp| LCD[(LCD)]
    D <-..->|fetch 'follow' > timestamp| LCD[(LCD)]
```
