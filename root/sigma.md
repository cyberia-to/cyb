---
tags: page, prysm, cyb
crystal-type: entity
crystal-domain: cyber
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