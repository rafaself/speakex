import { existsSync } from "node:fs";
import { spawnSync } from "node:child_process";
import process from "node:process";

const TOOLBOX_NAME = "speakex-dev";

function shellQuote(value) {
  return `'${value.replace(/'/g, `'\\''`)}'`;
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    stdio: "inherit",
    ...options,
  });

  if (result.error) {
    throw result.error;
  }

  return result.status ?? 1;
}

function isInsideContainer() {
  return Boolean(process.env.container) || existsSync("/run/.containerenv");
}

function hasToolboxContainer() {
  const result = spawnSync(
    "toolbox",
    ["run", "-c", TOOLBOX_NAME, "bash", "-lc", "true"],
    { stdio: "ignore" },
  );

  return result.status === 0;
}

const tauriArgs = process.argv.slice(2);
const shouldDisableStrip =
  process.platform === "linux" &&
  tauriArgs[0] === "build" &&
  (process.env.NO_STRIP === undefined || process.env.NO_STRIP === "");
const shouldUseToolbox =
  process.platform === "linux" &&
  process.env.SPEAKEX_SKIP_TOOLBOX !== "1" &&
  !isInsideContainer();

if (shouldUseToolbox) {
  if (!hasToolboxContainer()) {
    console.error(
      `SpeakEx expects the Fedora Toolbox container "${TOOLBOX_NAME}" on Linux hosts. ` +
        `Create or start it, or run this command from inside that container.`,
    );
    process.exit(1);
  }

  const command = [
    `cd ${shellQuote(process.cwd())}`,
    "&&",
    "PATH=\"$PWD/node_modules/.bin:$PATH\"",
    ...(shouldDisableStrip ? ["NO_STRIP=1"] : []),
    "SPEAKEX_SKIP_TOOLBOX=1",
    "tauri",
    ...tauriArgs.map(shellQuote),
  ].join(" ");

  process.exit(run("toolbox", ["run", "-c", TOOLBOX_NAME, "bash", "-lc", command]));
}

process.exit(
  run("tauri", tauriArgs, {
    env: {
      ...process.env,
      ...(shouldDisableStrip ? { NO_STRIP: "1" } : {}),
    },
  }),
);
