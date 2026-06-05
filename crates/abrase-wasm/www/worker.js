import init, { run, check, example_names, example_source, version } from "./pkg/abrase_wasm.js";

await init();

postMessage({
  type: "ready",
  version: version(),
  examples: example_names().map((name) => ({ name, source: example_source(name) })),
});

onmessage = (e) => {
  const { action, source } = e.data;
  const f = action === "check" ? check : run;
  const t0 = performance.now();
  const r = f(source);
  postMessage({
    type: "result",
    action,
    ms: performance.now() - t0,
    result: {
      ok: r.ok,
      value: r.value,
      stdout: r.stdout,
      stderr: r.stderr,
      error: r.error,
      warnings: r.warnings,
    },
  });
};
