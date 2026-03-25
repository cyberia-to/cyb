// Modular Lithium contract addresses (deployed on Bostrom 2026-03-04)
export const LITIUM_CORE_CONTRACT = 'bostrom1y9dqawhtk0m3sgglh2jgeu9y5zq5vmh5udmnw2unsm6j0j2nrskqe00ulm';
export const LITIUM_MINE_CONTRACT = 'bostrom123wr6faa62xxrft6t5wmpqmh9g0chvu7ddedggx0lkecmgef7thsls9my2';
export const LITIUM_STAKE_CONTRACT = 'bostrom1yagpj5dmr9fxj7qs08kdz2cpptf9va7jqwgx5257qjul2z6yq46sslqruy';
export const LITIUM_REFER_CONTRACT = 'bostrom1yvf9a2w6ydr79c4ufaj6wmk6sdw6xmct6ztn600chadjzek8639s34qxft';
export const LITIUM_WRAP_CONTRACT = 'bostrom1r5e285vff6mdyzhnh2aprcf3k9dujtjk5qqg30mqpd09cqde7tds3ue902';

export const LI_DENOM = `factory/${LITIUM_WRAP_CONTRACT}/li`;

export const UHASH_RELAY_URL = 'https://bostrom.cybernode.ai/relay';

// Min milliseconds between proof submissions (roughly one Bostrom block)
export const SUBMIT_COOLDOWN_MS = 6_000;

// cyb-boot distribution server (on cyberproxy, serves cyb.ai)
// Use relative URL so same-origin works in both dev (localhost) and production (cyb.ai)
export const BOOT_SERVER_URL = '/api/boot';
