---
tags: page
crystal-type: entity
crystal-domain: cyber
---
table render of [[cyb/brain]]

3 indicators in the head

- left:
	- amount of avatars particles
	- battery like indicator displaying % of discovery: `avatar particles / cybergraph particles`
- center: total size of brain in bytes
- right:
	- amount of avatars cyberlinks
	- battery like indicator displaying % of discovery: `avatar cyberlinks / cybergraph cyberlinks`

table of [[particles]] cyberlinked by active [[neuron]] sorted by [[cyberank]] by default

during surfing all ranks must be stored locally and updated during next visit

unique particles with last seen ranks is the source of raw list

fields

- [[spark]]: render of [[particle]]
- [[creator]]: [[neuron]] with first [[cyberlink]]
- [[cyb/time]]: how long time ago created by active [[neuron]]
	- not the first who [[cyberlink]]
- [[size]]: amount of bytes - must provide pin / unpin action
- [[probability of observation]]
- [[cyb/views]]: sum of incoming and outgoing cyberlinks to [[particle]]
	- on hover: who saw this particle? 5 random avatars in
		- choose the algorithm which is least intensive: random, last, top, etc.

table must be

- sortable by [[creator]], [[cyb/time]], [[size]], [[probability of observation]], [[cyb/views]] in both directions
- at least 21 rows on the screen
	- managing 1000 positions is impossible if i see 7-10 on screen
- view index
	- every page during surfing must store last index of row and state of analytics bar
	- must change parameter in url
	- i must be able to see exact position fast in previous view after hitting back

actions

- default button: add one particle
	- on add create a cyberlink [[particle]] -> `like`
- multilink
	- choose several particles
	- button for cyberlink with two options: in or out
- aka delete particle
	- create cyberlink: [[particle]] -> `delete`
	- do not display deleted particles except on `delete` particle page globally
- LATER: custom sorting