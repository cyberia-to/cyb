---
tags: cyb, core
crystal-type: entity
crystal-domain: cyber
---
# com

the command interface of [[cyb]]. where [[neurons]] express intent — push buttons, make decisions, ask questions, issue commands

com is the input surface. every other core app ([[cyb/brain]], [[cyb/oracle]], [[cyb/sigma]]) receives and displays. com is where the neuron acts

## command palette

`⌘K` opens the palette. type anything:

- a query → routes to [[cyb/oracle]]
- a page name → opens in [[cyb/brain]]
- a command → executes immediately
- a CID → resolves the [[particle]]
- an address → opens the [[neuron]] profile

fuzzy matching. recent commands. context-aware suggestions from the current view

## actions

every action in [[cyb]] flows through com:

| action | what happens |
|--------|-------------|
| link | create a [[cyberlink]] between two [[particles]] |
| send | transfer [[tokens]] via [[cyb/sigma]] |
| sign | approve a transaction with [[cyb/signer]] |
| publish | push a [[particle]] to the [[cybergraph]] |
| stake | delegate to a subnet |
| ask | submit a query to [[cyb/oracle]] |
| navigate | open a page in [[cyb/brain]] |

actions are composable. a single command can chain: search → select → link → publish

## keyboard-driven

com is designed for speed. every action has a keybinding. mouse is optional

- `⌘K` command palette
- `/` search in [[cyb/oracle]]
- `⌘L` new [[cyberlink]]
- `⌘S` sign pending transaction
- `⌘P` publish current [[particle]]
- `Tab` cycle between [[cyb/brain]] tabs
- `Esc` back / close / cancel

## voice and text input

com accepts natural language. the [[cyb/onnx]] SLM parses intent from free text and maps it to structured commands. say "stake 100 CYB on subnet 3" → com resolves it to a delegation transaction and routes to [[cyb/signer]]

## context awareness

com adapts to where you are:

- in [[cyb/brain]]: file operations, link creation, tab switching
- in [[cyb/oracle]]: search refinement, result selection, learn actions
- in [[cyb/sigma]]: token operations, staking, transfers
- in [[cyb/portal]]: onboarding steps, avatar creation

the palette shows only relevant commands for the current context

see [[cyb/core]] for how com fits among the nine core apps. see [[cyb/robot]] for the autonomous counterpart that acts without keyboard input