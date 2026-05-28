---
alias: my
tags: cell, prysm, cyb
crystal-type: pattern
crystal-domain: cyber
---

neuron dashboard — the personal space of a [[neuron]]

offline value: opens [[great web]] access

- core
	- through the lense of [[file]] and [[link]]
	- plan for future and understand the past using [[cyb/time]]
	- store [[particles]] in [[drive]]
	- solve [[tasks]]
		- first task is to sync top 1000
		- from [[cyber-sdk]] networks in [[hub]]
	- configure [[spells]]
	- explore your [[cyb/brain]]
	- explore you [[cyb/sense]]
	- improve you [[mind]]
	- intro to [[cyb/oracle]]
	- [[cyb/portal]]?
- configure [[neurons]]
- sync your [[nodes]] using local network

online value

- [[buy energy]]: agi access
- [[create avatars]] for talks with you
- explore and impact endless [[cyber]] using [[cyb/brain]]
- publish, distribute and promote [[files]] in [[cyb/sense]]
- optimize portfolio with [[cyb/sigma]]
- plan for future and understand the past using [[cyb/time]]
- sync your [[nodes]] using global network
- [[cyb/time]] line of external interactions

localhost:

- ipfs gateway
- ipfs api
- brain

[[avatars]], and [[progs]]

give access to [[cyb/state]]

gives dedicated [[neuron]] for each [[device]]

supports basic operations on [[signals]]

replicate [[state]] across [[devices]]

allow to add [[cyb/features]] to [[cyb/mind]]

superfeature: ability to act as a group of [[avatars]], [[neurons]] and [[progs]]

## pages

- [[cyb/brain]]
- [[cyb/time]]
- [[mind]]
- [[cyb/sigma]]
- [[cyb/sense]]
- [[map]]

## features

- core
	- [[cyb/sigma]]: valuation engine of [[tokens]]
	- [[cyb/sense]]: communication system of robot
	- [[cyb/time]]: memory of actions and planing system
	- [[cyb/brain]]: graph file manager
	- [[mind]]: decision engine
- features
	- TODO [[avatars]]: configurator of actors
	- TODO [[dreams]]: configure the most cherished wishes
	- TODO [[cyb/root]]: decision configurator
	- TODO [[values]]: configurator of optimization goals expressed in [[tokens]]
	- TODO [[neurons]]: configurator of [[signers]]
	- [[spells]]: creation, learning and storage of [[secrets]]
	- [[soul]]: one file configuration of your [[robot]], [[avatars]] and [[inference]]
	- TODO [[params]]: parameters configuration
	- TODO [[models]]: configure access to llms
	- TODO [[cryptor]]: sign, verify, encrypt, decrypt
	- TODO [[caster]]: [[signal]] handler
	- [[drive]]: private and public file system for [[cyb/brain]]
	- TODO [[tasks]]: executing particles and its status
	- [[nodes]]: configuration of physical devices of robot
	- TODO [[access]]: permission system for [[cells]]
	- [[network]]: configuration of connections
	- [[bridges]]: configure how to move [[value]] between networks
	- [[query]]: sophisticated [[cyb/brain]] analytics engine
	- [[debug]]: tools for making cyb and cyber better
	- [[about]]: information about software
	- TODO [[languages]]: configure semantics of your thoughts
	- TODO [[location]]: access to geolocation
	- TODO [[interfaces]]: configure input and output devices
	- TODO [[battery]]: access to node electric energy
	- TODO [[mouth]]: manage how robot speaks
	- TODO [[ears]]: configure access to microphones
	- TODO [[vision]]: connection to cameras
	- TODO [[projection]]: manage displays

## prysm cell

cell in the element tree $\mathcal{T}$. renders inside space zone of [[prysm/grid]]. accessible through avatar zone or by navigating to /@neuron_name

### sizing

fill × fill (occupies entire space zone)

### persistent header

always visible across all sub-pages:

```
glass [fill × auto, depth midground]
  stack horizontal [gap 2g]
    --- identity ---
    stack vertical [align center]
      glass [fix(8g) × fix(8g), corner-radius 4g] — avatar image (circle)
      text [body, "cybergirl.moon"]
      text [micro, "level 1"]
    --- address ---
    stack vertical [align center]
      address [big, with hash bars and sound]
      text [caption, "765 days"] — age in machine time
```

### stats sidebar

left column, present on all sub-pages. each row = link to corresponding sub-page:

```
stack vertical [fix(20g) × auto, gap g]
  ion + text + counter [Log, 0 tweets]
  ion + text + counter [Energy, 0 watt]
  ion + text + counter [Swarm, 0 learners]
  ion + text + counter [Security, 20 reward]
  ion + text + counter [Badges, 0 tokens]
  ion + text + counter [Karma, 0]
  ion + text + counter [Soul, 0]
```

tap item → navigates to that sub-page. active item highlighted

### left navigation menu

```
stack vertical [gap g/2]
  ion + text [main]
  ion + text [sense]
  ion + text [brain]
  ion + text [time]
  ion + text [sigma]
```

### sub-pages

#### main

neuron profile overview. header + stats sidebar + feed area

```
--- right side ---
glass [fill × auto, depth midground]
  text [body, "no feeds"] — or feed of neuron's cyberlinks
```

commander shows "enter password to unlock" + "Unlock"

#### Log (tweets)

feed of neuron's published [[particles]] — cyberlinks created by this neuron

#### Energy

energy dashboard for this neuron — personal view of [[cyb/reactor]]

```
--- formula ---
stack horizontal [gap g]
  glass [fix(8g) × fix(8g), depth midground, emotion green tint]
    counter [h2, "0 W"]
    text [caption, "Energy"]
  text [h2, "+"]
  glass [fix(8g) × fix(6g), depth midground]
    counter [h2, "1 764 W"]
    text [caption, "Income"]
  text [h2, "-"]
  glass [fix(8g) × fix(6g), depth midground]
    counter [h2, "0 W"]
    text [caption, "Outcome"]
  text [h2, "="]
  glass [fix(8g) × fix(6g), depth midground]
    counter [h2, "1 764 W"]
    text [caption, "Free Energy"]

text [body, "Energy (W) is the product of amperes and volts"]

--- balance ---
text [h3, "Balance:"]
stack horizontal [gap g]
  glass: counter [0 A (ampere)]
  text [h2, "×"]
  glass: counter [0 V (volt)]
  text [h2, "="]
  glass: counter [0 W]

--- rod table ---
tabs [State ▲ | Unfreezing ▲ | Supplied ▲ | Received ▲]
table or "no data"
```

#### Swarm

social connections — friends, following, followers

```
stack vertical [gap 3g]
  --- Friends ---
  text [h3, "Friends"]
  grid [avatar icons of mutual connections] or "no friends"
  --- Following ---
  text [h3, "Following"]
  grid [avatar icons] or "no following"
  --- Followers ---
  text [h3, "Followers"]
  grid [avatar icons] or "no followers"
```

#### Security

personal staking — my heroes (validators I delegate to)

```
table [sortable]
  columns: Validator ▲ | Unbondings ▲ | Rewards ▲ | Amount ▲
  rows: delegations to validators
```

#### Badges

earned tokens/NFTs — reputation markers

```
table [sortable]
  columns: Discipline ▲ | TOCYB ▲ | BOOT ▲
  rows: badge data or "no data"
```

#### Karma

reputation score visualization (under construction — mushroom placeholder)

future: karma breakdown, history, rank position

#### Time

full transaction history for this neuron

```
table [sortable]
  columns: status ▲ (✓/✗) | type ▲ (icon + label) | timestamp ▲ | tx (hash, green, link) | action
```

action column expands inline for IBC details, send/receive, cyberlinks

#### Sigma (inside robot)

personal token balances scoped to this neuron

#### Soul

cybscript editor — programmable neuron behavior

```
toggle [cybscript enabled]
glass [fill × auto, depth midground]
  text [code editor, monospace, syntax highlighted]
```

commander shows "test cybscript" + "reset to default"

### fold

$\mathcal{F}$:
- $l_1$ ($w_{min} = 40g$): header + sidebar + content side by side
- $l_2$ ($w_{min} = 20g$): header stacked, sidebar above content
- $l_3$ ($w_{min} = 10g$, mobile): compact header, sidebar collapsed to icons, content full width

### emotion

| element | emotion | trigger |
|---------|---------|---------|
| stats counters | green if > 0, white if 0 | value state |
| Security row | green (has rewards) | reward pending |
| Energy formula | green tint on Energy box | energy available |
| address hash bars | full acid palette | address identity |

### states

| state | visual | trigger |
|-------|--------|---------|
| locked | "enter password to unlock" in commander | wallet not connected |
| unlocked | full functionality | password entered |
| viewing own | edit capabilities (soul editor, customize) | own neuron |
| viewing other | read-only profile | another neuron's address |

### ECS

- Entity: robot-cell organelle
- Components:
  - `Sizing { width: Fill, height: Fill }`
  - `Overflow { scroll }`
  - `FoldSet { conformations }`
  - `ActiveSubPage { main | log | energy | swarm | security | badges | karma | soul }`
  - `NeuronIdentity { address, name, level, age, avatar_cid }`
  - `NeuronStats { log_count, energy_watt, swarm_learners, security_reward, badges_tokens, karma, soul }`
  - `IsOwnNeuron { bool }` — determines edit capabilities
- Systems:
  - `RobotMenuSystem` handles sidebar navigation
  - `RobotStatsSystem` fetches neuron stats
  - `RobotEnergySystem` fetches energy balance
  - `RobotSwarmSystem` fetches social graph (friends, following, followers)
  - `RobotSecuritySystem` fetches delegations
  - `RobotSoulSystem` handles cybscript editing, testing, deployment
