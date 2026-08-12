# CYB local backend(in-browser)

Cyb plays singinficat role in cyber infrastructure. The app reconstruct self-sufficient backend+frontend pattern inside the browser.
In big view app consist from 3 parts:

```mermaid
graph TD;
        App["frontend\n(main thread)"]-.proxy.->Backend["backend\n(shared worker)"];
        App-.proxy.->Db["graph db\n(shared worker)"];
        Backend-.proxy.->Db;
        App<-.message\nchannels.->Backend;
```

To reduce overload of main thread we have created 2 separate shared workers, where all the stuff is hosted. Bi-interraction between all layers occurs using proxy(comlink abstraction) or directly using broadcast channels.

## Db layer

Db worker is pretty simple it it's host only local relational-graph-vector database - [[cozo]]. It's represented with DbApi in frontend and backend layers.
Cozo provide bazing fast access to brain and ipfs data in relational form and also in vector format, processing by [ml]embedder.

```mermaid
graph TD;
        dbApi["dbApi"]--odb_meta_orm;
        subgraph rune["cozo db"]
            db_meta_orm[["meta orm"]]-.->db;
        end
```

### Db entities

- brain:
  - particles
    - embeddings
  - links
  - transactions
  - community
- sense:

  - sync items + update status

- system:
  - config
  - queue messages

## Backend layer

Backend worker is more complicated it contains significant elements of cyb architecture:

```mermaid
graph TD;
    subgraph Backend["backend(shared worker)"]

        subgraph ipfs["ipfs implementations"]
            helia;
            kubo;
            js-ipfs;
        end

        subgraph queues["message brokers"]
            ipfs_queue["ipfs load balancer"];
            queue["data processing queue aka bus"];
        end

        subgraph rune["rune"]
            vm["virtual machine"]--ovm_bingen{{"cyb bindings"}};
        end

        subgraph sense["sense"]
            link_sync["link sync"];
            msg_sync["message sync"];
            swarm_sync["swarm sync"];
        end

        subgraph ml["ML transformers"]
            feature_extractor["embedder"];
        end

    end
```

### Ipfs module

Represented with IpfsApi at frontend layer, but also have direct access for some edge cases

- Uses module that encapsulate different Ipfs implementations(kubo, helia, js-ipfs(obsolete))
  - cache content(local storage & cozo)
  - preserve redundancy
- Ipfs queue, process all requests to ipfs, prioritize, cancel non-actual requests and organize content pipeline
  - responsible for:
    - ipfs load balancing(limit of requests)
    - request prioritizing(actual requests first)
    - fault processing(switch fetch policy)
    - post processing(**inline rune vm** into pipeline)

```mermaid
graph LR
user(ipfsApi\nenqueue particle) --> q[["queue\n(balancer)"]] --> node[/"ipfs"/] -- found --> rune[rune vm] -- mutation | content --> cache["cache"] --> app(app\ncontent)
node -. not found\n(retry | error) .-> q
```

## Bus

Represented with some helpers and used for cases when blaancer is needed, some services not initialized yet(deffered actions), or long calculations is requered(ml inference, ipfs requests):

- particle, request ipfs, save; calc embedding
- link, deffered save
- message persistence is protected by db store

```mermaid
graph TD;
        sender{{"enqueue(...)"}} -.message bus.-> bus
        subgraph task["task manager"]
            bus[["queue listener"]];

            bus-.task.->db("store\ndata")--odb1["dbApi"];
            bus-.task.->ml("calculate\nembedding")--oml1["mlApi"];
            bus-.task.->ipfs("request ipfs\nlow-priority")--oi["ipfsApi"]
        end
```

## Sense

Represented by SenseApi + subscription to broadcast channel at fronted layer. Provide continious update of cyberlinks related to my brain and my swarm, recieving on chain messages etc.:

- Particles service (pooling)
- Transactions service (pooling + websocket)
- My friends service (pooling)
- Ipfs service(pooling)

All data and update status is stored into db, when some new data is recieved that triggers notification for frontendю

```mermaid
graph TD;
    db[["dbApi"]];
    bus[["particle queue"]];

    subgraph sense["sync service"]
        notification("notification service")

        particles[["particle service"]]--onotification;
        transactions[["transaction service"]]--onotification;
        myfriend[["my friends service"]]--onotification;

        particles -.loop.-> particles;
        transactions -.loop.-> transactions;
        myfriend -.loop.-> myfriend;
    end


    subgraph blockchain["blockchain"]
        lcd[["lcd"]]
        websockets("websockets")
        indexer[["indexer"]]
    end

    subgraph app["frontend"]
        redux["redux"]
        sender{{"senseApi"}};
    end

    notification -.message.-> redux;
    sender -.proxy.-> db;
    sense -.proxy.-> db;
    sense -.message.-> bus;
    bus -.proxy.-> db;

    sense <-.request\nsubscriptin.->blockchain;

```

## Rune

Rune VM execution is pipelined thru special abstraction called entrypoints. VM have bindings to all app parts: DB, transformers, signer, blockchain api, ipfs and also includes context of the entrypoint.(see. [[scripting]] for detailed description).

## ML transformers

Represented my mlApi. Uses inference from local ML models hosted inside browser.

- future extractor. BERT-like model to trnsform text-to-embeddings.

```mermaid
graph TD;
    subgraph ml["transformers"]
        embedder["embedder"];
    end

    subgraph dbApi["dbApi"]
        db[["DB"]];
    end
    mlApi["mlApi"];

    mlApi--odb;
    mlApi--oembedder;
```
