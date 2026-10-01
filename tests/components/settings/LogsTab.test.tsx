import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { LogsTab } from "../../../src/components/settings/LogsTab";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), updateSetting: vi.fn(), showAlert: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("lucide-react", async () => await vi.importActual("lucide-react"));
vi.mock("react-i18next", () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
vi.mock("../../../src/hooks/useSettings", () => ({ useSettings: () => ({ settings: { loggingEnabled: true, maxLogEntries: 1000 }, updateSetting: mocks.updateSetting }) }));
vi.mock("../../../src/hooks/useAlert", () => ({ useAlert: () => ({ showAlert: mocks.showAlert }) }));

beforeEach(() => {
  vi.clearAllMocks();
  mocks.invoke.mockImplementation(async (command: string) => {
    if (command === "get_logs") return ["INFO", "ERROR", "WARN", "DEBUG"].map((level) => ({ timestamp: "12:00", level, message: `${level} message` }));
    if (command === "get_log_settings") return { enabled: true, max_size: 1000, current_count: 4 };
    throw new Error(`Unexpected command: ${command}`);
  });
});

describe("LogsTab theme controls", () => {
  it("uses theme tokens for refresh, filters, toggle and semantic log levels", async () => {
    render(<LogsTab />);
    await waitFor(() => expect(screen.getByRole("button", { name: "settings.refreshLogs" })).toBeEnabled());
    expect(screen.getByRole("button", { name: "settings.refreshLogs" })).toHaveClass("bg-accent-primary", "hover:bg-accent-primary/90", "text-inverse");
    expect(screen.getByRole("combobox")).toHaveClass("focus:border-focus");
    expect(screen.getByRole("checkbox").nextElementSibling).toHaveClass("peer-checked:bg-accent-primary");
    for (const [level, tone] of Object.entries({ INFO: "info", ERROR: "error", WARN: "warning", DEBUG: "success" })) {
      expect(await screen.findByText(level, { selector: "span" })).toHaveClass(`text-accent-${tone}`);
    }
    expect(mocks.invoke.mock.calls.every(([command]) => command === "get_logs" || command === "get_log_settings")).toBe(true);
  });
});
