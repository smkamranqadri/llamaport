import { useCallback, useEffect, useState } from "react";
import { openPath } from "@tauri-apps/plugin-opener";
import { agentConnection, piInspect, piPreview } from "./api";
import type { AgentConnectionInfo, PiInspection, PiPreview } from "./types";

function Copyable({ label, value }: { label: string; value: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="fact">
      <dt>{label}</dt>
      <dd>
        <span className="copyable">
          <code>{value}</code>
          <button
            className="link-stop"
            onClick={() => {
              void navigator.clipboard.writeText(value);
              setCopied(true);
              setTimeout(() => setCopied(false), 1500);
            }}
          >
            {copied ? "copied" : "copy"}
          </button>
        </span>
      </dd>
    </div>
  );
}

function Block({
  title,
  path,
  body,
}: {
  title: string;
  path: string;
  body: string;
}) {
  const [copied, setCopied] = useState(false);
  return (
    <div className="preview-block">
      <div className="panel-head">
        <span className="field-hint">
          {title} — paste into <code>{path}</code>
        </span>
        <button
          className="button"
          onClick={() => {
            void navigator.clipboard.writeText(body);
            setCopied(true);
            setTimeout(() => setCopied(false), 1500);
          }}
        >
          {copied ? "Copied" : "Copy"}
        </button>
      </div>
      <pre className="command">{body}</pre>
    </div>
  );
}

export default function Connect() {
  const [info, setInfo] = useState<AgentConnectionInfo | null>(null);
  const [inspection, setInspection] = useState<PiInspection | null>(null);
  const [preview, setPreview] = useState<PiPreview | null>(null);
  const [failure, setFailure] = useState<string | null>(null);

  const refresh = useCallback(() => {
    agentConnection().then(setInfo).catch((e) => setFailure(String(e)));
  }, []);

  useEffect(() => {
    refresh();
    const timer = setInterval(refresh, 4000);
    return () => clearInterval(timer);
  }, [refresh]);

  const connection = info?.connection ?? null;
  const configuredPort = inspection?.localProvider?.baseUrl?.match(/:(\d+)/)?.[1];
  const portMismatch =
    connection != null &&
    configuredPort != null &&
    Number(configuredPort) !== connection.port;

  return (
    <>
      <header className="screen-header">
        <div>
          <h1>Connect</h1>
          <p className="screen-subtitle">
            Point Pi, Picot or any OpenAI-compatible client at the running server
          </p>
        </div>
      </header>

      {failure && <p className="notice notice-error">{failure}</p>}

      {!connection ? (
        <div className="empty">
          <p className="empty-title">No model running</p>
          <p className="empty-detail">
            Start one from the Library and the endpoint details appear here.
          </p>
        </div>
      ) : (
        <section className="panel">
          <h2>Endpoint</h2>
          <dl className="facts">
            <Copyable label="OpenAI-compatible URL" value={connection.openaiUrl} />
            <Copyable label="Base URL" value={connection.baseUrl} />
            <Copyable label="Model alias" value={connection.alias} />
            <div className="fact">
              <dt>Binding</dt>
              <dd>
                {connection.loopbackOnly ? (
                  <span className="badge safety-green">localhost only</span>
                ) : (
                  <span className="badge safety-red">reachable off this machine</span>
                )}
              </dd>
            </div>
            <div className="fact">
              <dt>Health</dt>
              <dd>{info?.healthy ? "serving" : "not ready"}</dd>
            </div>
            <div className="fact">
              <dt>Authentication</dt>
              <dd>none — anyone on this machine can use it</dd>
            </div>
          </dl>
        </section>
      )}

      <section className="panel">
        <div className="panel-head">
          <h2>Pi configuration</h2>
          <div className="actions">
            <button className="button" onClick={() => piInspect().then(setInspection)}>
              Inspect my Pi setup
            </button>
            <button
              className="button"
              disabled={!connection}
              onClick={() =>
                piPreview()
                  .then(setPreview)
                  .catch((e) => setFailure(String(e)))
              }
            >
              Generate configuration
            </button>
          </div>
        </div>

        <p className="field-hint">
          This app never writes your Pi files. It generates text for you to paste,
          and reads your configuration only when you press Inspect.
        </p>

        {inspection && (
          <>
            <dl className="facts">
              <div className="fact">
                <dt>settings.json</dt>
                <dd>{inspection.settingsFound ? "found" : "not found"}</dd>
              </div>
              <div className="fact">
                <dt>models.json</dt>
                <dd>{inspection.modelsFound ? "found" : "not found"}</dd>
              </div>
              <div className="fact">
                <dt>Providers</dt>
                <dd>{inspection.providerNames.join(", ") || "none"}</dd>
              </div>
              <div className="fact">
                <dt>Local provider</dt>
                <dd>{inspection.localProvider?.name ?? "none"}</dd>
              </div>
              <div className="fact">
                <dt>Points at</dt>
                <dd>{inspection.localProvider?.baseUrl ?? "—"}</dd>
              </div>
              <div className="fact">
                <dt>API key set</dt>
                <dd>{inspection.localProvider?.hasApiKey ? "yes" : "no"}</dd>
              </div>
            </dl>
            {inspection.notes.map((note) => (
              <p key={note} className="field-hint">
                {note}
              </p>
            ))}
          </>
        )}

        {portMismatch && (
          <p className="notice notice-error">
            Pi is configured for port {configuredPort} but the server is running on{" "}
            {connection?.port}. Requests from Pi will not reach it until one of them
            changes.
          </p>
        )}

        {preview && (
          <>
            <Block
              title="Provider entry"
              path={preview.modelsPath}
              body={preview.provider}
            />
            <Block
              title="Model selection"
              path={preview.settingsPath}
              body={preview.settings}
            />
          </>
        )}
      </section>

      <section className="panel">
        <h2>Open</h2>
        <div className="actions">
          {(info?.apps ?? []).map((app) => (
            <button
              key={app.path}
              className="button"
              onClick={() => void openPath(app.path).catch((e) => setFailure(String(e)))}
            >
              {app.name}
            </button>
          ))}
          {info?.sessionsDir && (
            <button
              className="button"
              onClick={() =>
                void openPath(info.sessionsDir!).catch((e) => setFailure(String(e)))
              }
            >
              Pi session folder
            </button>
          )}
        </div>
        {(info?.apps ?? []).length === 0 && (
          <p className="field-hint">
            No known applications found in /Applications.
          </p>
        )}
      </section>
    </>
  );
}
