/**
 * Скрипт для пининга иконок токенов и сетей на Pinata.
 *
 * Запуск:
 *   DENO_NO_PACKAGE_JSON=1 deno run --allow-net --allow-read --allow-env scripts/pin-icons-to-pinata.ts
 *
 * Требует .env с:
 *   PINATA_API_KEY, PINATA_API_SECRET, PINATA_GATEWAY
 */

const LCD_URL = "https://lcd.bostrom.cybernode.ai";
const HUB_TOKENS = "bostrom15phze6xnvfnpuvvgs2tw58xnnuf872wlz72sv0j2yauh6zwm7cmqqpmc42";
const HUB_NETWORKS = "bostrom1lpn69a74ftv04upfej8f9ay56pe2zyk48vzlk49kp3grysc7u56qq363nr";

// --- Load .env ---
async function loadEnv(): Promise<Record<string, string>> {
  const env: Record<string, string> = {};
  try {
    const text = await Deno.readTextFile(".env");
    for (const line of text.split("\n")) {
      const trimmed = line.trim();
      if (!trimmed || trimmed.startsWith("#")) continue;
      const eqIdx = trimmed.indexOf("=");
      if (eqIdx > 0) {
        env[trimmed.slice(0, eqIdx)] = trimmed.slice(eqIdx + 1);
      }
    }
  } catch {
    console.error("Не найден файл .env");
    Deno.exit(1);
  }
  return env;
}

// --- Query smart contract ---
async function queryContract(contractAddr: string): Promise<any> {
  const queryMsg = btoa(JSON.stringify({ get_entries: {} }));
  const url = `${LCD_URL}/cosmwasm/wasm/v1/contract/${contractAddr}/smart/${queryMsg}`;
  const resp = await fetch(url);
  if (!resp.ok) throw new Error(`Query failed: ${resp.status} ${await resp.text()}`);
  const json = await resp.json();
  return json.data;
}

// --- Pin CID on Pinata by hash ---
async function pinByHash(
  cid: string,
  name: string,
  apiKey: string,
  apiSecret: string
): Promise<boolean> {
  const resp = await fetch("https://api.pinata.cloud/pinning/pinByHash", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      pinata_api_key: apiKey,
      pinata_secret_api_key: apiSecret,
    },
    body: JSON.stringify({
      hashToPin: cid,
      pinataMetadata: { name },
    }),
  });

  if (resp.ok) {
    return true;
  }

  const text = await resp.text();
  // Already pinned
  if (text.includes("DUPLICATE_OBJECT") || text.includes("already being tracked")) {
    return true;
  }

  console.error(`  Ошибка пина ${cid}: ${resp.status} ${text}`);
  return false;
}

// --- Main ---
async function main() {
  const env = await loadEnv();
  const apiKey = env.PINATA_API_KEY;
  const apiSecret = env.PINATA_API_SECRET;
  const gateway = env.PINATA_GATEWAY;

  if (!apiKey || !apiSecret) {
    console.error("PINATA_API_KEY и PINATA_API_SECRET нужны в .env");
    Deno.exit(1);
  }

  console.log("Загружаю токены из контракта...");
  const tokensData = await queryContract(HUB_TOKENS);
  const tokens = tokensData.entries || [];

  console.log("Загружаю сети из контракта...");
  const networksData = await queryContract(HUB_NETWORKS);
  const networks = networksData.entries || [];

  // Collect unique CIDs
  const cids = new Map<string, string>(); // cid -> name
  for (const token of tokens) {
    if (token.logo) {
      cids.set(token.logo, `token-${token.ticker}`);
    }
  }
  for (const network of networks) {
    if (network.logo) {
      cids.set(network.logo, `network-${network.name || network.chain_id}`);
    }
  }

  console.log(`\nНайдено ${cids.size} уникальных CID иконок\n`);

  let pinned = 0;
  let failed = 0;

  for (const [cid, name] of cids) {
    console.log(`Пиню: ${name} (${cid})`);
    const ok = await pinByHash(cid, name, apiKey, apiSecret);
    if (ok) {
      pinned++;
      if (gateway) {
        console.log(`  ✓ https://${gateway}/ipfs/${cid}`);
      } else {
        console.log(`  ✓ запинено`);
      }
    } else {
      failed++;
    }
  }

  console.log(`\nГотово: ${pinned} запинено, ${failed} ошибок`);
  if (gateway) {
    console.log(`Gateway: https://${gateway}/ipfs/<CID>`);
  }
}

main();
