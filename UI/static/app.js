const state = {
  running: false,
};

const elements = {
  binaryPath: document.querySelector("#binary-path"),
  repoRoot: document.querySelector("#repo-root"),
  defaultKeyPath: document.querySelector("#default-key-path"),
  platform: document.querySelector("#platform"),
  status: document.querySelector("#run-status"),
  summary: document.querySelector("#command-summary"),
  timestamp: document.querySelector("#command-timestamp"),
  output: document.querySelector("#command-output"),
  forms: Array.from(document.querySelectorAll(".command-form")),
  revokeAll: document.querySelector("#revoke-all"),
  revokeIds: document.querySelector("#revoke-key-ids"),
};

document.addEventListener("DOMContentLoaded", () => {
  bindForms();
  bindToggles();
  loadMeta();
});

function bindForms() {
  for (const form of elements.forms) {
    form.addEventListener("submit", handleSubmit);
  }
}

function bindToggles() {
  if (!elements.revokeAll || !elements.revokeIds) {
    return;
  }

  const syncRevokeState = () => {
    const disableIds = elements.revokeAll.checked;
    elements.revokeIds.disabled = disableIds;
    if (disableIds) {
      elements.revokeIds.placeholder = "Serial numbers are not needed when revoke all is enabled.";
    } else {
      elements.revokeIds.placeholder = "Paste one or more serial numbers, separated by spaces, commas, or new lines.";
    }
  };

  elements.revokeAll.addEventListener("change", syncRevokeState);
  syncRevokeState();
}

async function loadMeta() {
  try {
    const response = await fetch("/api/meta");
    if (!response.ok) {
      throw new Error(`metadata request failed with status ${response.status}`);
    }

    const meta = await response.json();
    elements.binaryPath.textContent = meta.binaryPath;
    elements.repoRoot.textContent = meta.repoRoot;
    elements.defaultKeyPath.textContent = meta.defaultKeyPath;
    elements.platform.textContent = meta.platform;

    for (const input of document.querySelectorAll("[data-default-key-path]")) {
      if (!input.value) {
        input.value = meta.defaultKeyPath;
      }
    }
  } catch (error) {
    renderFailure(`Failed to load UI metadata: ${error.message}`);
  }
}

async function handleSubmit(event) {
  event.preventDefault();

  if (state.running) {
    return;
  }

  const form = event.currentTarget;
  const label = form.dataset.label || "command";
  const payload = buildPayload(form);

  setRunning(true);
  elements.status.dataset.state = "pending";
  elements.status.textContent = "Running";
  elements.summary.textContent = `${label} is running. If authentication is needed, finish it in the browser window opened by cscs-key.`;
  elements.timestamp.textContent = new Date().toLocaleString();
  elements.output.textContent = JSON.stringify(payload, null, 2);

  try {
    const response = await fetch("/api/run", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify(payload),
    });

    const data = await response.json().catch(() => ({ message: "Unexpected server response." }));

    if (!response.ok) {
      throw new Error(data.message || `Request failed with status ${response.status}`);
    }

    renderResult(data);
  } catch (error) {
    renderFailure(error.message);
  } finally {
    setRunning(false);
  }
}

function buildPayload(form) {
  const formData = new FormData(form);
  const command = form.dataset.command;
  const payload = {
    command,
  };

  const env = stringValue(formData.get("env"));
  if (env) {
    payload.env = env;
  }

  if (command === "gen" || command === "sign") {
    const file = stringValue(formData.get("file"));
    const duration = stringValue(formData.get("duration"));

    if (file) {
      payload.file = file;
    }

    if (duration) {
      payload.duration = duration;
    }
  }

  if (command === "list") {
    payload.all = checkboxValue(formData, "all");
  }

  if (command === "revoke") {
    payload.all = checkboxValue(formData, "all");
    payload.dry = checkboxValue(formData, "dry");

    const ids = stringValue(formData.get("keyIds"))
      .split(/[\s,]+/)
      .map((value) => value.trim())
      .filter(Boolean);

    if (ids.length > 0) {
      payload.keyIds = ids;
    }
  }

  return payload;
}

function renderResult(result) {
  elements.status.dataset.state = result.ok ? "success" : "failure";
  elements.status.textContent = result.ok ? "Success" : "Failed";
  elements.summary.textContent = result.commandLine;
  elements.timestamp.textContent = `Finished at ${new Date().toLocaleString()}${formatExitCode(result.exitCode)}`;
  elements.output.textContent = formatOutput(result.stdout, result.stderr);
}

function renderFailure(message) {
  elements.status.dataset.state = "failure";
  elements.status.textContent = "Error";
  elements.summary.textContent = message;
  elements.timestamp.textContent = `Updated at ${new Date().toLocaleString()}`;
  elements.output.textContent = message;
}

function formatOutput(stdout, stderr) {
  const chunks = [];

  if (stdout && stdout.trim()) {
    chunks.push(`STDOUT\n${stdout.trimEnd()}`);
  }

  if (stderr && stderr.trim()) {
    chunks.push(`STDERR\n${stderr.trimEnd()}`);
  }

  return chunks.length > 0 ? chunks.join("\n\n") : "Command finished with no output.";
}

function setRunning(running) {
  state.running = running;
  for (const button of document.querySelectorAll("button")) {
    button.disabled = running;
  }
}

function stringValue(value) {
  return typeof value === "string" ? value.trim() : "";
}

function checkboxValue(formData, name) {
  return formData.get(name) === "on";
}

function formatExitCode(exitCode) {
  if (exitCode === null || exitCode === undefined) {
    return "";
  }

  return ` with exit code ${exitCode}`;
}