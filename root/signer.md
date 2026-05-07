---
tags: cyb
crystal-type: entity
crystal-domain: cyber
status: DONE
---
2 basic computations on [[particles]]

signal preparation is responsibility of [[particle]]

signal broadcast is responsibility of [[cyb/caster]]

TODO fn sign

- [[neuron]] [[sign]] [[particles]] using [[spells]]
- input
	- particle
	- scheme
		- ecdsa
		- schnorr
		- bls
	- curve
		- secp256k1
		- sr25519
		- ed25519
		- [bls12-381](https://hackmd.io/@benjaminion/bls12-381)
	- path
	- address format
	- pubkey to address
	- dictionary
		- magic words from bitcoin
		- monero derived words
		- raw private key
- output
	- signature as [[particle]]
	- [[neuron]]

TODO fn verify

- input
	- neuron
	- signature
	- particle
- output
	- true or false

thoughts

- universal signer does not exist
	- analysis of [[public signers]]
	- plugable signatures
	- plugable curves
	- plugable derivation paths
	- plugable dictionaries
- authentic confirmation screen for signing messages is crucial for safety
	- background must be secretly and deterministically generated
- social recovery is important
	- shamir secret sharing for splitting secrets
- dapp to wallet interactions are important
	- how to make wallet connect without servers?

| Blockchain       | Signing Scheme                    | Curve(s)            | Notes                                                                 |
|------------------|-----------------------------------|---------------------|-----------------------------------------------------------------------|
| Bitcoin (BTC)    | ECDSA                             | secp256k1           | -                                                                     |
| Ethereum (ETH)   | ECDSA (execution layer)           | secp256k1           | Uses ECDSA for transaction signing in the execution layer.            |
| Ethereum 2.0     | BLS (consensus layer)             | BLS12-381           | Uses BLS for validator signatures in the consensus layer.             |
| Polkadot         | SR25519, ED25519, ECDSA           | sr25519, ed25519, secp256k1 | Offers multiple schemes for flexibility and interoperability.          |
| Solana           | EdDSA                             | ed25519             | Chosen for speed and security, crucial for Solana's performance.      |
| Cosmos Ecosystem | ECDSA, EdDSA                      | secp256k1, ed25519  | Supports multiple schemes for versatility across Cosmos-based chains. |


| Blockchain | Standard Derivation Path    | Coin Type | Remarks                                                |
|------------|-----------------------------|-----------|--------------------------------------------------------|
| Bitcoin    | `m/44'/0'/0'/0/x`           | `0'`      | BIP44 standard, `x` for address index                  |
| Ethereum   | `m/44'/60'/0'/0/x`          | `60'`     | Used for generating multiple addresses                 |
| Solana     | `m/44'/501'/0'/0'`          | `501'`    | Typically uses a single address, no address index (`x`)|
| Polkadot   | `m/44'/354'/0'/0'/x'`       | `354'`    | Account and address indexing, `x'` for address index   |
| Cosmos     | `m/44'/118'/0'/0/x`         | `118'`    | Similar to Bitcoin and Ethereum, `x` for address index |

Yes, you've got the right idea. BIP44 and similar standards like BIP32 and BIP43 define the hierarchical structure for deriving public and private keys from a master seed, not the addresses directly. The process for generating an address from a public key can vary significantly between different blockchain networks, even if they use the same derivation path to generate the public key. Essentially, the derivation path leads to a public/private key pair, and then each blockchain network applies its own rules and formats to generate an address from the public key.

This distinction is crucial because it underlines that the same hierarchical path (e.g., `m/44'/0'/0'/0/0`) could generate the same public/private key pair across different networks following the BIP44 standard. However, the method each network uses to convert the public key into a blockchain-specific address can lead to completely different addresses on those networks.

Let's build a comparison table on the specifics of address computation across different blockchains like Bitcoin, Ethereum, Solana, Polkadot, and Cosmos to illustrate this further.

| Blockchain | Public Key Derivation | Address Computation                                                                                   |
|------------|-----------------------|-------------------------------------------------------------------------------------------------------|
| Bitcoin    | BIP32/BIP44           | Bitcoin addresses are either Pay-to-Public-Key-Hash (P2PKH) or Pay-to-Script-Hash (P2SH), starting with '1' or '3', respectively. SHA-256 followed by RIPEMD-160 is applied to the public key. |
| Ethereum   | BIP32/BIP44           | Ethereum uses the 'keccak-256' hash of the public key, taking the last 20 bytes to form the address, prefixed with '0x'. |
| Solana     | BIP32/BIP44           | Solana addresses are derived by taking the SHA-256 hash of the public key and using the Base58 encoding, similar to Bitcoin but without the version byte and checksum. |
| Polkadot   | BIP32/BIP44           | Polkadot uses the SS58 address format. It involves hashing the public key and encoding it with a network-specific prefix. |
| Cosmos     | BIP32/BIP44           | Cosmos addresses use the 'sha256' hash of the public key, followed by RIPEMD-160. The result is base32 encoded with the Bech32 format, prefixed with the network's unique identifier (e.g., 'cosmos'). |

| Feature               | Bitcoin                  | Ethereum                 | Solana                   | Polkadot                | Cosmos                   |
|-----------------------|--------------------------|--------------------------|--------------------------|-------------------------|--------------------------|
| Primary Cryptography | SHA-256 & RIPEMD-160     | Keccak-256               | Ed25519                  | Schnorrkel (sr25519) & Ed25519 | Secp256k1 & Ed25519      |
| Address Format    | Base58Check encoding     | Hex, prefixed with `0x`  | Base58                   | SS58                    | Bech32                    |
| Public Key to Address | Double hash (SHA-256 then RIPEMD-160) of the public key, add network byte, checksum with Base58Check | Hash the public key with Keccak-256 and take the last 20 bytes | Use the public key directly as the address, encode with Base58 | Public key is hashed and encoded using the SS58 format, which includes a prefix indicating the network | Hash the public key with SHA-256 (for Secp256k1) or directly use the public key (for Ed25519), then encode with Bech32 |
| Unique Characteristics | Uses a checksum for error detection. Addresses can start with `1`, `3` or bech32 `bc1` | Addresses are not checksum-cased by default, but EIP-55 proposes a mixed-case checksum variant | Addresses are the shortest among these blockchains, offering efficiency | Polkadot's SS58 address format includes a network identifier, enabling address reuse across networks with safety | Cosmos utilizes the Bech32 address format, enhancing readability and error detection |

You're correct in noting that the construction of a transaction generally exists on a different layer from the signing process and indeed can be designed to operate without directly exposing keys from the signer. This separation between transaction construction and signing is a fundamental aspect of most blockchain architectures, enhancing security and modularity. Let's explore how this separation impacts the system's design and operation:

### Transaction Construction

Formation of Transaction Data: This involves creating a structured set of data that includes all necessary components of a transaction, such as sender and receiver addresses, amount to be transferred, and any additional data required by the specific blockchain protocol (like a nonce in Ethereum or a memo field in other blockchains).

Independence from Signing: Constructing a transaction doesn't require access to private keys; it's about forming a valid data structure according to the rules of the specific blockchain network. The transaction data at this stage is typically unsigned or contains placeholder values for signatures.

### Signing Process

Signing with Private Keys: Once a transaction is constructed, it needs to be signed with the sender's private key. This cryptographic process generates a signature proving that the sender has authorized the transaction. Importantly, this step can be performed without the signing mechanism needing to know the details of the transaction's intended actions, beyond what is necessary to calculate the signature.

No Exposure of Keys: The signing process can be and often is, handled by secure modules or hardware (like Hardware Security Modules (HSMs), secure enclaves, or hardware wallets). These tools are designed to sign data with a private key without exposing the key itself, even to the rest of the system performing the transaction construction.

### Benefits of Separation

Enhanced Security: Keeping the key management and signing processes separate from transaction construction minimizes the risk of private key exposure. It allows sensitive operations to be isolated, potentially within secure hardware or software environments.

Modularity: This separation allows for greater flexibility in system design. Different components can be updated or replaced independently (e.g., changing the transaction format for a specific blockchain without altering the signing mechanism).

Interoperability: A modular approach facilitates the development of systems that can interact with multiple blockchains. The same secure signing module can be used across different networks with diverse transaction structures, as long as it is provided with the correct data to sign.

### Application in Multi-Blockchain Environments

  In a system designed for universal key and address management across multiple blockchains, this separation allows the core signing mechanism to remain constant and secure. At the same time, the transaction construction layer can adapt to the specific requirements of each blockchain network. Such a design supports scalability and adaptability, crucial for managing transactions across an evolving landscape of blockchain technologies without compromising on security or flexibility.

Here's a comparison table that highlights the key differences between hardened and non-hardened (normal) addresses in the context of hierarchical deterministic (HD) wallets, as outlined in BIP32 and related standards:

| Feature                      | Hardened Addresses                                           | Non-Hardened Addresses                                       |
|------------------------------|--------------------------------------------------------------|--------------------------------------------------------------|
| Derivation Path Notation     | Denoted by an apostrophe (`'`) after the index number. E.g., `m/44'/0'/0'` | No apostrophe after the index number. E.g., `m/44/0/0`      |
| Key Generation Input         | Uses the parent's private key as part of the input for generating the child key. | Uses the parent's public key as part of the input for generating the child key. |
| Backtracking Security        | If a child private key is exposed, it does not compromise the parent private key or other sibling keys. | If a child private key and the parent chain code are exposed, it could potentially allow backtracking to the parent private key and compromise other sibling keys. |
| Public Key Derivation        | Cannot generate child public keys without access to the parent's private key. | Can generate child public keys without access to the parent's private key, allowing for a more flexible key management structure. |
| Address Generation Visibility | Requires access to the private key, limiting the ability to generate addresses in less secure environments without exposing the key. | Public keys can be derived and used to generate addresses in less secure environments without exposing the private key. |
| Use Case                     | Used for higher security needs, where exposure of a single key should not risk other keys. Ideal for savings or high-value accounts. | Used where convenience and efficient key management are prioritized, such as for generating receiving addresses on a server without needing access to the private keys. |

The choice between using hardened or non-hardened addresses depends on the specific security needs, operational requirements, and risk profile of the use case. Hardened addresses provide enhanced security against certain types of attacks, making them suitable for protecting high-value keys. In contrast, non-hardened addresses offer more flexibility in key management, allowing for public key derivation and address generation without exposing private keys.

No, knowing some parent keys and sibling keys in a hardened derivation structure does not allow you to derive other private keys, including those of the siblings, due to the nature of hardened key derivation. Let's clarify why this is the case:

### The Nature of Hardened Derivation

Hardened Key Generation: In the hardened key derivation process (as specified by BIP32 and its derivatives), each child key is generated using the parent's private key and an index number that indicates a hardened derivation (usually denoted by an index >= 2^31). Importantly, this process also involves the use of the parent's chain code in a way that ensures the derived child key cannot be used to backtrack to the parent's private key or to any other siblings' private keys.

Security by Design: The specific cryptographic operation that generates a hardened child key (using HMAC-SHA512 in the case of BIP32) is designed so that knowing a child key (even if it's a public key) does not reveal information about the parent's private key or allow the derivation of sibling keys. The operation uses the parent's private key as part of its input, but the output (the child key) does not expose any information that would allow reversing the process.

### Implications of Knowing Some Keys

No Backward Derivation: If you know a parent public key and a child public key from a hardened derivation, you cannot derive the parent private key or any sibling keys. The hardened derivation process is specifically designed to prevent this to secure the wallet against such potential attacks.

Isolation of Key Pairs: Each hardened key pair is effectively isolated from its siblings. Knowledge of one hardened child key pair does not compromise the others, nor does it compromise the parent key pair. This isolation is a critical feature for maintaining the security and integrity of hierarchical deterministic wallets.

### Summary

  In summary, the hardened derivation process in HD wallets ensures that even if you know some parent and sibling keys, you cannot derive other private keys within the structure. This security feature protects against potential attacks where an actor might gain access to a subset of keys and attempts to derive others. The design of hardened keys specifically guards against backward derivation and ensures that each key pair remains secure and isolated within the hierarchical structure.

Your outlined approach for a universal signer does indeed present a universal and adaptable framework capable of supporting a wide range of blockchain networks and their varying requirements for transaction signing. By abstracting the key components necessary for cryptographic operations and addressing across multiple blockchains, you set a foundation that can accommodate existing technologies as well as adapt to future developments. Let's explore how this framework could incorporate various digital signature (DS) schemes, curves, address formats, pubkey to address algorithms, and dictionaries, and consider its application to multi-signature setups and other advanced features.

### Incorporating Diverse DS Schemes and Curves

Modular DS Scheme Integration: By designing the signer to accept different DS schemes (e.g., ECDSA, Schnorr, BLS) as plug-in modules or configurable options, the system can easily adapt to the specific requirements of each blockchain. This modular approach allows for the addition of new schemes as they are developed and adopted by the community.

Curve Flexibility: Similar to DS schemes, the system can support a variety of elliptic curves (e.g., secp256k1, sr25519, ed25519, bls12-381) by implementing them as interchangeable components. This ensures compatibility with a broad range of blockchain protocols and enhances the system's ability to leverage the unique security and performance characteristics of each curve.

### Handling Various Address Formats and Algorithms

Configurable Address Format and Algorithm: The framework can include a library of address formats and the corresponding algorithms for converting public keys into blockchain-specific addresses. By allowing these components to be specified per transaction or network configuration, the signer remains flexible and universally applicable.

### Supporting Diverse Dictionaries

Dictionary Support for Mnemonics: Including support for multiple dictionaries (e.g., BIP39 for Bitcoin, Monero's dictionary) ensures that the signer can work with mnemonic phrases from various blockchains, enhancing its usability for key recovery and backup processes.

### Multi-Signature and Transaction Construction

External Transaction Construction: By keeping the transaction construction, including multi-signature configurations, outside the core protocol of the universal signer, the signer maintains focus on its primary function: signing. This separation of concerns simplifies the signer's design and allows it to remain blockchain-agnostic.

Multi-Signature Support: The framework can accommodate multi-signature transactions by ensuring it can sign messages or transactions as part of a multi-signature scheme. The actual logic for constructing multi-signature transactions and assembling signatures from multiple signers would be handled by external modules tailored to each blockchain's specific requirements.

### Universal Approach Validity

Comprehensive and Adaptable: This approach provides a comprehensive foundation for a universal signer, capable of adapting to the diverse and evolving landscape of blockchain technology. It supports a wide array of cryptographic primitives and operational requirements across different blockchains.

Future-Proofing: By designing the system with modularity and configurability at its core, it can easily incorporate new DS schemes, curves, address formats, and other advancements in blockchain technology, ensuring its long-term relevance and utility.

  In summary, your proposed framework for a universal signer is well-conceived, offering a flexible and scalable solution for managing cryptographic operations across multiple blockchain networks. Its modular design and focus on core signing capabilities, while delegating transaction-specific logic to external components, make it a robust and versatile tool for blockchain applications.

Your approach to derivation paths introduces a novel and user-centric framework for organizing keys within a hierarchical deterministic (HD) wallet. It aims to balance the technical requirements of key management with the practical needs of users, making it more intuitive to manage and use keys across different contexts. Let's analyze each component of your proposed derivation path structure:

### m / avatar' / neuron' / particle' / invoice'

Avatar: Represents a broad category or identity within the user's ecosystem, such as personal, business, or specific projects. This level allows users to segregate and manage their financial and operational activities under distinct identities, enhancing both organizational clarity and security.

Neuron: Encompasses specific devices or organizational units under an avatar. This could include different devices (mobile, desktop), environments (home, work), or even specific blockchain contracts. Neurons enable users to manage access and permissions finely, tailoring key usage to specific devices or contexts.

Particle: Targets application-specific usage, allowing keys to be allocated and exposed to particular applications. This level of granularity supports secure application interactions, ensuring that apps access only the keys they are permitted to use, thereby reducing risk.

Invoice: Facilitates the management of incoming payments, enabling the generation of unique identifiers for transactions. This could streamline payment tracking and reconciliation, especially useful in business or service contexts where transaction management is critical.

### Analysis of Usability and Flexibility

  Pros:

Intuitive Organization: The hierarchy mirrors real-world organizational structures, making it more intuitive for users to manage their keys and understand their wallet's structure.

Enhanced Security: By segregating keys across different levels (avatar, neuron, particle), users can limit exposure and minimize risk. If a key at the particle level is compromised, the impact is contained within that specific application context.

Adaptability: This structure is flexible enough to accommodate a wide range of use cases, from individual users managing personal and work finances to organizations overseeing complex structures of departments, devices, and applications.

  Cons:

Complexity for Casual Users: While offering significant advantages in organization and security, the structure might be overly complex for casual users who may not need such granularity in key management.

Implementation Challenge: Wallets and systems implementing this structure would need to develop user interfaces and support mechanisms that help users navigate and utilize the hierarchy effectively, without becoming overwhelmed by the options.

Standardization and Compatibility: For this approach to gain widespread adoption, it would require some level of standardization or compatibility with existing HD wallet standards (like BIP32, BIP44). Integrating such a novel structure into the broader ecosystem could present challenges.

### Conclusion

  Your approach to a more usable derivation path structure is innovative and user-focused, offering significant potential benefits in terms of security, organization, and adaptability. It reflects a deep understanding of the needs of diverse users and use cases within the cryptocurrency and blockchain space. However, its success would depend on careful implementation, user education, and potentially, efforts towards standardization to ensure broad compatibility and ease of use. The proposed structure demonstrates a promising direction for making key management more accessible and tailored to the complex needs of users in a rapidly evolving digital asset landscape.

TODO cast

- [[cyb/avatar]] send [[messages]] as [[particles]] to other [[cyb/avatar]]
- either store locally
- or broadcast to endpoint
	- dependency on [[hub]]