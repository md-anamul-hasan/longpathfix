import { execFileSync } from "node:child_process";
import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import { createRequire } from "node:module";
import { fileURLToPath } from "node:url";

const require = createRequire(
  path.resolve(
    path.dirname(fileURLToPath(import.meta.url)),
    "../src-ui/package.json"
  )
);
const { chromium } = require("playwright");

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const DIST_DIR = path.join(ROOT, "src-ui", "dist");
const REVIEW_DIR = path.join(ROOT, ".impeccable", "review");
const SHOTS_DIR = path.join(ROOT, "docs", "screenshots");
const IMPECCABLE_CMD =
  "C:\\Users\\anamm\\.gemini\\config\\plugins\\impeccable\\skills\\impeccable\\scripts\\impeccable.cmd";

fs.mkdirSync(REVIEW_DIR, { recursive: true });
fs.mkdirSync(SHOTS_DIR, { recursive: true });

const MIME = {
  ".html": "text/html; charset=utf-8",
  ".js": "application/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".json": "application/json",
};

function startStaticServer() {
  return new Promise((resolve) => {
    const server = http.createServer((req, res) => {
      const reqPath = (req.url || "/").split("?")[0];
      const safeRel = path.normalize(reqPath === "/" ? "/index.html" : reqPath);
      const filePath = path.join(DIST_DIR, safeRel);
      if (!filePath.startsWith(DIST_DIR) || !fs.existsSync(filePath)) {
        res.writeHead(404);
        res.end("Not found");
        return;
      }
      const ext = path.extname(filePath);
      res.writeHead(200, { "Content-Type": MIME[ext] || "application/octet-stream" });
      fs.createReadStream(filePath).pipe(res);
    });
    server.listen(0, "127.0.0.1", () => {
      const addr = server.address();
      resolve({ server, url: `http://127.0.0.1:${addr.port}` });
    });
  });
}

function buildMockInitScript() {
  return () => {
    const callbacks = new Map();
    let nextCbId = 1;
    const listeners = new Map();

    const state = {
      config: {
        theme: "dark",
        defaultLimit: 259,
        customLimit: 220,
        defaultFixKinds: 31,
        clearReadonlyWhenAuthorized: false,
        recentRoots: [
          "D:\\Archives\\macOS-Migration-2026",
          "C:\\Users\\anam\\OneDrive\\Client-Handover",
        ],
      },
      policy: {
        longPathsEnabled: false,
        registryValue: 0,
        processLongPathAware: true,
        isElevated: false,
        osBuild: 26300,
        classicMaxPath: 259,
        extendedMaxPath: 32767,
      },
      volumes: [
        {
          rootPath: "C:\\",
          volumeLabel: "System",
          filesystemName: "NTFS",
          maxComponentLength: 255,
          supportsUnicodeOnDisk: true,
          supportsReparsePoints: true,
          supportsCaseSensitiveDirs: true,
          isReadOnly: false,
          driveType: "Fixed drive",
        },
        {
          rootPath: "D:\\",
          volumeLabel: "Projects",
          filesystemName: "NTFS",
          maxComponentLength: 255,
          supportsUnicodeOnDisk: true,
          supportsReparsePoints: true,
          supportsCaseSensitiveDirs: true,
          isReadOnly: false,
          driveType: "Fixed drive",
        },
      ],
      findings: [
        {
          node: {
            rel: "clients\\acmecorp\\2026\\infrastructure\\migrations\\multi-region-failover\\playbooks\\quarterly-dr-exercise-runbook-final-reviewed-draft-with-appendix-v12\\implementation-notes\\deeply-nested-verification-checklist-for-production-cutover-and-post-incident-audit-report-2026.pdf",
            isDirectory: false,
            isReparsePoint: false,
            isReadonly: false,
            invalidUnicode: false,
          },
          issues: 1,
          newName: "deeply-nested-verifica~1b4a.pdf",
          unresolved: null,
          currentLen: 304,
          plannedLen: 238,
          issueText: "Path > 259 (304)",
        },
        {
          node: {
            rel: "exports\\mac-backup\\notes: 2026-10-22?.md",
            isDirectory: false,
            isReparsePoint: false,
            isReadonly: true,
            invalidUnicode: false,
          },
          issues: 32,
          newName: "notes_ 2026-10-22_.md",
          unresolved: null,
          currentLen: 81,
          plannedLen: 81,
          issueText: "Illegal characters (<>:\"/\\|?*)",
        },
        {
          node: {
            rel: "exports\\dos-devices\\CON.json",
            isDirectory: false,
            isReparsePoint: false,
            isReadonly: false,
            invalidUnicode: false,
          },
          issues: 16,
          newName: "_CON.json",
          unresolved: null,
          currentLen: 69,
          plannedLen: 70,
          issueText: "Reserved Windows name",
        },
        {
          node: {
            rel: "exports\\mac-backup\\release-summary . ",
            isDirectory: true,
            isReparsePoint: false,
            isReadonly: false,
            invalidUnicode: false,
          },
          issues: 4,
          newName: "release-summary",
          unresolved: null,
          currentLen: 72,
          plannedLen: 69,
          issueText: "Trailing space or dot",
        },
        {
          node: {
            rel: "exports\\linux-sync\\Readme.md",
            isDirectory: false,
            isReparsePoint: false,
            isReadonly: false,
            invalidUnicode: false,
          },
          issues: 64,
          newName: "Readme_2.md",
          unresolved: null,
          currentLen: 68,
          plannedLen: 70,
          issueText: "Case-insensitive clash",
        },
      ],
      summary: {
        rootPlain: "D:\\Archives\\macOS-Migration-2026",
        rootExtended: "\\\\?\\D:\\Archives\\macOS-Migration-2026",
        rootLength: 33,
        limit: 259,
        kinds: 31,
        scanned: 14280,
        scannedFiles: 12910,
        scannedDirs: 1370,
        skippedReparsePoints: 4,
        readonlyFindings: 1,
        findingsCount: 5,
        opsCount: 5,
        unresolvedCount: 0,
        maxLenBefore: 304,
        maxLenAfter: 238,
        elapsedMs: 42,
        warningsCount: 0,
        warnings: [],
        issueCounts: {
          tooLong: 1,
          nameTooLong: 0,
          trailingDotOrSpace: 1,
          leadingSpace: 0,
          reservedName: 1,
          illegalChars: 1,
          caseClash: 1,
          invalidUnicode: 0,
        },
        histogram: [
          { label: "0-120", min: 0, max: 120, before: 8420, after: 8420 },
          { label: "121-200", min: 121, max: 200, before: 4310, after: 4310 },
          { label: "201-240", min: 201, max: 240, before: 1120, after: 1550 },
          { label: "241-259", min: 241, max: 259, before: 290, after: 0 },
          { label: "260-320", min: 260, max: 320, before: 140, after: 0 },
          { label: "321+", min: 321, max: 99999, before: 0, after: 0 },
        ],
      },
      history: [
        {
          id: "20261114-091205-a19c",
          time: "2026-11-14T09:12:05",
          displayTime: "2026-11-14 09:12:05",
          root: "D:\\Archives\\macOS-Migration-2026",
          count: 14,
          undone: false,
          title: "Renamed 14 items (macOS-Migration-2026)",
        },
        {
          id: "20261112-174410-f032",
          time: "2026-11-12T17:44:10",
          displayTime: "2026-11-12 17:44:10",
          root: "C:\\Users\\anam\\OneDrive\\Client-Handover",
          count: 3,
          undone: true,
          title: "Renamed 3 items (Client-Handover)",
        },
      ],
    };

    function emit(eventName, payload) {
      const ids = listeners.get(eventName) || [];
      for (const id of ids) {
        const cb = callbacks.get(id);
        if (cb) cb({ event: eventName, id, payload });
      }
    }

    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
      unregisterListener(eventName, id) {
        const arr = listeners.get(eventName) || [];
        listeners.set(
          eventName,
          arr.filter((x) => x !== id)
        );
      },
    };

    window.__TAURI_INTERNALS__ = {
      metadata: {
        currentWindow: { label: "main" },
        currentWebview: { windowLabel: "main", label: "main" },
      },
      transformCallback(callback) {
        const id = nextCbId++;
        callbacks.set(id, callback);
        return id;
      },
      async invoke(cmd, args = {}) {
        switch (cmd) {
          case "plugin:event|listen": {
            const arr = listeners.get(args.event) || [];
            arr.push(args.handler);
            listeners.set(args.event, arr);
            return args.handler;
          }
          case "plugin:event|unlisten":
            return null;
          case "get_bootstrap":
            return {
              version: "2.1.1",
              config: state.config,
              policy: state.policy,
              volumes: state.volumes,
              dataDir: "C:\\Users\\anam\\AppData\\Roaming\\LongPathFix",
              historyCount: state.history.length,
            };
          case "set_config":
            state.config = args.config;
            return state.config;
          case "inspect_root":
            return {
              valid: true,
              plain: args.rootInput || state.config.recentRoots[0],
              extended: `\\\\?\\${args.rootInput || state.config.recentRoots[0]}`,
              len16: (args.rootInput || state.config.recentRoots[0]).length,
              warning: null,
              volume: state.volumes[1],
              error: null,
            };
          case "start_scan":
            setTimeout(() => emit("scan-complete", state.summary), 30);
            return null;
          case "update_plan_options":
            state.summary.limit = args.limit;
            state.summary.kinds = args.kinds;
            return { ...state.summary };
          case "query_findings":
            return {
              totalMatching: state.findings.length,
              fixableMatching: state.findings.length,
              items: state.findings,
            };
          case "inspect_path": {
            const rel = args.rel || state.findings[0].node.rel;
            const found =
              state.findings.find((f) => f.node.rel === rel) || state.findings[0];
            const segs = found.node.rel.split("\\");
            return {
              rel: found.node.rel,
              plainPath: `${state.summary.rootPlain}\\${found.node.rel}`,
              extendedPath: `${state.summary.rootExtended}\\${found.node.rel}`,
              isDirectory: found.node.isDirectory,
              isReparsePoint: false,
              isReadonly: found.node.isReadonly,
              isLocked: false,
              size: found.node.isDirectory ? null : 2481920,
              currentLen: found.currentLen,
              plannedLen: found.plannedLen,
              limit: state.summary.limit,
              issueLabels: [found.issueText],
              unresolved: null,
              segments: [
                {
                  name: state.summary.rootPlain,
                  newName: null,
                  utf16Len: state.summary.rootLength,
                  newUtf16Len: state.summary.rootLength,
                  issues: [],
                  isRoot: true,
                },
                ...segs.map((name, idx) => ({
                  name,
                  newName: idx === segs.length - 1 ? found.newName : null,
                  utf16Len: name.length,
                  newUtf16Len:
                    idx === segs.length - 1 && found.newName
                      ? found.newName.length
                      : name.length,
                  issues: idx === segs.length - 1 ? [found.issueText] : [],
                  isRoot: false,
                })),
              ],
            };
          }
          case "validate_custom_name":
            return {
              valid: true,
              utf16Len: (args.name || "").length,
              issues: [],
              error: null,
            };
          case "preview_repair":
            return {
              items: state.findings.map((f) => ({
                rel: f.node.rel,
                oldName: f.node.rel.split("\\").pop(),
                newName: f.newName,
                isDirectory: f.node.isDirectory,
                status: f.node.isReadonly ? "read_only" : "ready",
                message: f.node.isReadonly
                  ? "File is Read-only"
                  : "Ready to rename in-place",
              })),
              readyCount: 4,
              readonlyCount: 1,
              conflictCount: 0,
              lockedCount: 0,
              missingCount: 0,
              blockedCount: 0,
            };
          case "get_system_diagnostics":
            return {
              policy: state.policy,
              volumes: state.volumes,
              dataDir: "C:\\Users\\anam\\AppData\\Roaming\\LongPathFix",
              historyDir: "C:\\Users\\anam\\AppData\\Roaming\\LongPathFix\\history",
              logFile: "C:\\Users\\anam\\AppData\\Roaming\\LongPathFix\\logs\\longpathfix.jsonl",
            };
          case "analyze_path_input":
            return {
              input: args.input,
              form: "drive",
              normalizedPlain: args.input,
              extendedPath: `\\\\?\\${args.input}`,
              utf16Len: (args.input || "").length,
              exceedsClassic: (args.input || "").length > 259,
              existsOnDisk: false,
              isDirectory: null,
              isReadonly: null,
              isReparsePoint: null,
              segments: [
                { name: "C:", newName: null, utf16Len: 2, newUtf16Len: 2, issues: [], isRoot: true },
                { name: "CON", newName: "_CON", utf16Len: 3, newUtf16Len: 4, issues: ["Reserved Windows name"], isRoot: false },
                { name: "quarterly report .txt", newName: "quarterly report.txt", utf16Len: 21, newUtf16Len: 20, issues: [], isRoot: false },
              ],
              error: null,
            };
          case "get_history":
            return state.history;
          case "get_history_batch_detail":
            return {
              Id: args.id,
              Time: "2026-11-14T09:12:05",
              Root: "D:\\Archives\\macOS-Migration-2026",
              Undone: false,
              Done: [
                {
                  Dir: "exports\\mac-backup",
                  OldName: "report:2026?.pdf",
                  NewName: "report_2026_.pdf",
                  IsDirectory: false,
                },
                {
                  Dir: "exports\\dos-devices",
                  OldName: "PRN.log",
                  NewName: "_PRN.log",
                  IsDirectory: false,
                },
              ],
            };
          default:
            return null;
        }
      },
    };
  };
}

async function launchBrowser() {
  for (const channel of ["msedge", "chrome", undefined]) {
    try {
      return await chromium.launch({
        headless: true,
        ...(channel ? { channel } : {}),
        args: ["--remote-debugging-port=9229"],
      });
    } catch {
      // try next browser
    }
  }
  throw new Error("Could not launch Chromium/Edge via Playwright.");
}

function runCdpDetect(label) {
  try {
    const out = execFileSync(
      `"${IMPECCABLE_CMD}" detect --cdp 9229 --json`,
      [],
      { encoding: "utf8", shell: true }
    );
    console.log(`${label}:`, out.trim());
  } catch (err) {
    if (err && typeof err.status === "number" && err.status === 2) {
      console.log(`${label} (findings):`, (err.stdout || "").toString().trim());
    } else {
      throw err;
    }
  }
}

async function main() {
  const { server, url } = await startStaticServer();
  const browser = await launchBrowser();
  const errors = [];

  try {
    const context = await browser.newContext({
      viewport: { width: 1280, height: 800 },
      deviceScaleFactor: 1,
    });
    const page = await context.newPage();
    page.on("pageerror", (e) => errors.push(`pageerror: ${e.message}`));
    page.on("console", (msg) => {
      if (msg.type() === "error") errors.push(`console.error: ${msg.text()}`);
    });

    await page.addInitScript(buildMockInitScript());
    await page.goto(url, { waitUntil: "networkidle" });

    // Trigger Scan to populate PathGauge + Findings Table
    await page.getByRole("button", { name: "Scan Folder" }).click();
    await page.waitForSelector(".histogram-grid");
    await page.waitForSelector(".data-table tbody tr td.col-check");
    await page.waitForTimeout(80);

    // 1. Desktop Scan & Fix Dark (1280x800)
    await page.screenshot({ path: path.join(SHOTS_DIR, "scan-fix-dark.png") });
    runCdpDetect("CDP_DETECT_DARK");

    // Assert zero horizontal overflow at 1280x800
    const overflow1280 = await page.evaluate(
      () => document.documentElement.scrollWidth > window.innerWidth
    );
    if (overflow1280) throw new Error("Horizontal overflow detected at 1280x800");

    // 2. Inspector Dark + Review Desktop (1440x900)
    await page.locator(".data-table tbody tr").first().click();
    await page.waitForSelector(".inspector-drawer");
    await page.screenshot({ path: path.join(SHOTS_DIR, "inspector-dark.png") });

    await page.setViewportSize({ width: 1440, height: 900 });
    await page.screenshot({ path: path.join(REVIEW_DIR, "desktop.png") });

    // 3. Light theme Scan & Fix (1280x800)
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.getByRole("button", { name: "Light" }).click();
    await page.getByRole("button", { name: "Close path inspector" }).click();
    await page.waitForTimeout(150);
    await page.screenshot({ path: path.join(SHOTS_DIR, "scan-fix-light.png") });
    runCdpDetect("CDP_DETECT_LIGHT");

    // Switch back to Dark for modal & secondary views
    await page.getByRole("button", { name: "Dark" }).click();
    await page.waitForTimeout(120);

    // 4. Dry-run Repair Preview Modal
    await page.getByRole("button", { name: /Preview & Fix/ }).click();
    await page.waitForSelector(".modal-panel");
    await page.screenshot({ path: path.join(SHOTS_DIR, "dry-run-modal-dark.png") });
    await page.getByRole("button", { name: "Close repair dialog" }).click();

    // 5. Path Lab & System Diagnostics
    await page.getByRole("button", { name: "Path Lab & System" }).click();
    await page.waitForSelector("#path-lab-input");
    await page.waitForTimeout(250);
    await page.screenshot({ path: path.join(SHOTS_DIR, "diagnostics-dark.png") });

    // 6. History & Undo (with expanded detail)
    await page.getByRole("button", { name: "History & Undo" }).click();
    await page.waitForSelector(".history-item");
    await page.getByRole("button", { name: "Inspect entries" }).first().click();
    await page.waitForSelector(".history-detail");
    await page.screenshot({ path: path.join(SHOTS_DIR, "history-dark.png") });

    // 7. Settings & Guide
    await page.getByRole("button", { name: "Settings & Guide" }).click();
    await page.waitForSelector("#pref-limit");
    await page.waitForTimeout(180);
    await page.screenshot({ path: path.join(SHOTS_DIR, "settings-guide-dark.png") });

    // 8. Mobile Viewport (390x844) -> .impeccable/review/mobile.png
    await page.getByRole("button", { name: "Scan & Fix" }).click();
    await page.setViewportSize({ width: 390, height: 844 });
    await page.waitForTimeout(150);
    const overflow390 = await page.evaluate(
      () => document.documentElement.scrollWidth > window.innerWidth
    );
    await page.screenshot({ path: path.join(REVIEW_DIR, "mobile.png") });
    if (overflow390) throw new Error("Horizontal overflow detected at 390x844");

    if (errors.length > 0) {
      throw new Error(`Console/Page errors:\n${errors.join("\n")}`);
    }
    console.log("UI_CAPTURE_OK");
  } finally {
    await browser.close();
    server.close();
  }
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
