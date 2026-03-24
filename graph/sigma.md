---
tags: page, prysm, cyb
crystal-type: entity
crystal-domain: cyber
stake: 18341118728537776
diffusion: 0.0003435934627465641
springs: 0.0006737345629141142
heat: 0.0005948356786094808
focus: 0.0004928842359694269
gravity: 10
density: 15.32
---

wallet and balance [[aip]] in [[cyb]]

widget molecule and full application in [[prysm]]

the economic interface between a [[neuron]] and the [[cybergraph]]

## interface

- inputs
	- token balances: [[CYB]], [[HYDROGEN]], [[BOOT]], [[VOLT]], [[AMPERE]], and IBC tokens
	- staking state: delegations, rewards, unbonding
	- portfolio value: aggregated across all [[token]] types
- outputs
	- send action → token transfer
	- stake action → delegation to subnet
	- navigate action → opens token detail or [[cyberver]]

## as widget (molecule)

- compact balance display in the [[prysm/hud]]
- shows total portfolio value as [[prysm/counter]]
- token breakdown on expand
- [[emotion]] color reflects portfolio trend (green rising, red falling)

## as aip

- full-screen token management
- pages
	- [[coins]]: fungible token balances and transfers
	- [[cards]]: unique tokens and collectibles
	- [[scores]]: reputation and contribution metrics
	- [[badges]]: achievement tokens
- with [[focus]] on [[value optimization]]