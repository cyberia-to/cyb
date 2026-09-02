cyb mvp — three-screen foundation

purpose

the three-screen mvp establishes the interactive core of cyb: identity creation, graph exploration, and local inference. each screen corresponds to a distinct mode of engagement with the cybergraph and its participants.

screens

spells

a neuron generates its on-chain identity here. the screen presents a 12-word bip39 mnemonic in a 4x3 pill grid and offers generation and import paths. each word occupies a glass(midground) surface labeled with its index. a generate action creates a fresh entropy-derived mnemonic. an import path accepts an existing phrase via a text input. the mnemonic becomes the seed material for neuron keys — it does not leave the local machine.

graph

the graph screen bridges cyb's worldstate fsm to mir's graphworldplugin. when worldstate::graph is active, mir receives graphworldstate::active and takes over the render surface via a fullscreen imagenode. the synthetic csr encodes 20 key cybergraph concepts with weighted directed edges. spectral layout, heat-kernel diffusion, t2/t3/tinf rendering passes, and edge flow animation run entirely within mir. the cyb shell provides only the csr and the state signal.

sense

a chat interface for local inference. messages accumulate in a scrollable glass(background) pane. user input travels to a local llm endpoint (ollama v1-compatible at localhost:11434) via a thread-pool task using ureq. responses appear as bot messages. the async model keeps the bevy frame loop unblocked. when the llm server is unavailable the screen surfaces a diagnostic message.

stack integration points

- prysm bevy module (cyb/bevy/src/prysm/) — design tokens, atoms, and molecules as bevy ecs components
- mir graphworldplugin — added to the app alongside graphbridgeplugin which manages state synchronization and csr construction
- bip39 crate — mnemonic generation in spells
- ureq crate — blocking http in a bevy asynccomputetaskpool thread for sense
- worldstate fsm — extended from 6 to 7 variants; hotkeys updated to cmd+1..7

prysm foundation

all three screens compose from the same prysm atoms and molecules: glass surfaces at four depth levels, acid emotion colors (grounded in the evolutionary [[color]] map), monospace typography scale, saber dividers, commander tabs, spawn_button, and spawn_input. visual consistency across screens follows from shared theme constants rather than per-screen style decisions.

build order

1. prysm module (tokens, atoms, molecules, plugin)
2. worldstate and hotkey updates
3. worlds/spells (uses only prysm + bip39)
4. worlds/graph (uses mir, builds synthetic csr)
5. worlds/sense (uses prysm + ureq + bevy tasks)
6. main.rs wiring
