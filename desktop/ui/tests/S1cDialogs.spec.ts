// S1c 向导/对话框组件测试：导入两阶段、导出口令校验、注入启动字段选择。
// api 全 mock（invoke 不触 Rust 侧）。
import { describe, expect, it, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import ImportWizard from "../src/components/ImportWizard.vue";
import ExportDialog from "../src/components/ExportDialog.vue";
import LaunchDialog from "../src/components/LaunchDialog.vue";

const api = vi.hoisted(() => ({
  importScan: vi.fn(),
  importCommit: vi.fn(),
  exportArchive: vi.fn(),
  launchWithEnv: vi.fn(),
  pickFile: vi.fn(),
  pickFolder: vi.fn(),
  entryGet: vi.fn(),
}));

vi.mock("../src/api", () => ({
  importScan: api.importScan,
  importCommit: api.importCommit,
  exportArchive: api.exportArchive,
  launchWithEnv: api.launchWithEnv,
  pickFile: api.pickFile,
  pickFolder: api.pickFolder,
  entryGet: api.entryGet,
}));

const SCAN = {
  total: 3,
  unique: 2,
  l1_duplicates: 1,
  l2_merged: 0,
  l3_conflicts: 0,
  manual_review: ["[L1 duplicate] a.env:3 Token"],
  candidates: [
    { title: "[dotenv/.env#1] a.env", notes: "source: .env:1 rule: token", password_len: 12 },
    { title: "[dotenv/.env#2] b.env", notes: "source: b.env:2 rule: token", password_len: 9 },
  ],
};

describe("ImportWizard", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("扫描 → 报告与候选表；确认提交并上抛 imported", async () => {
    api.importScan.mockResolvedValue(SCAN);
    api.importCommit.mockResolvedValue(2);
    const w = mount(ImportWizard);
    await w.find("#imp-kind").setValue("dotenv");
    await w.find("#imp-path").setValue("C:/tmp/.env");
    await w.find("button.primary").trigger("click");
    await flushPromises();
    expect(api.importScan).toHaveBeenCalledWith("dotenv", "C:/tmp/.env");
    expect(w.text()).toContain("候选 2");
    expect(w.text()).toContain("[dotenv/.env#1] a.env");
    expect(w.text()).not.toContain("password_len");
    // 确认提交（第二个 primary 按钮）
    await w.findAll("button.primary")[1].trigger("click");
    await flushPromises();
    expect(api.importCommit).toHaveBeenCalledWith("dotenv", "C:/tmp/.env");
    expect(w.text()).toContain("已导入 2 条");
    expect(w.emitted("imported")).toHaveLength(1);
    w.unmount();
  });

  it("env 类无需路径即可扫描", async () => {
    api.importScan.mockResolvedValue({ ...SCAN, candidates: [] });
    const w = mount(ImportWizard);
    await w.find("#imp-kind").setValue("env");
    await w.find("button.primary").trigger("click");
    await flushPromises();
    expect(api.importScan).toHaveBeenCalledWith("env", null);
    w.unmount();
  });
});

describe("ExportDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("口令为空/不一致时不调用导出", async () => {
    const w = mount(ExportDialog);
    await w.find("#exp-pw1").setValue("");
    await w.find("button.primary").trigger("click");
    expect(w.text()).toContain("归档口令不能为空");
    await w.find("#exp-pw1").setValue("sp-test-only-pw");
    await w.find("#exp-pw2").setValue("different");
    await w.find("button.primary").trigger("click");
    expect(w.text()).toContain("两次口令不一致");
    expect(api.exportArchive).not.toHaveBeenCalled();
    w.unmount();
  });

  it("口令一致 → 导出并展示路径与摘要", async () => {
    api.exportArchive.mockResolvedValue({
      path: "C:/out/sp-export.age",
      summary: { entries: 5, attachments: 1, history_total: 0, warnings: [] },
    });
    const w = mount(ExportDialog);
    await w.find("#exp-pw1").setValue("sp-test-only-pw");
    await w.find("#exp-pw2").setValue("sp-test-only-pw");
    await w.find("button.primary").trigger("click");
    await flushPromises();
    expect(api.exportArchive).toHaveBeenCalledWith("sp-test-only-pw", "sp-test-only-pw");
    expect(w.text()).toContain("C:/out/sp-export.age");
    expect(w.text()).toContain("5 条条目");
    w.unmount();
  });
});

describe("LaunchDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const DETAIL = {
    title: "Demo",
    username: "u1",
    password: "p1",
    url: "https://demo.local",
    notes: null,
    otp: null,
    custom_fields: [{ name: "api_key", value: "k-1", protected: false }],
    tags: [],
  };

  it("加载字段 → 勾选后启动（标准+自定义字段名传给后端）", async () => {
    api.entryGet.mockResolvedValue(DETAIL);
    api.launchWithEnv.mockResolvedValue(4321);
    const w = mount(LaunchDialog, { props: { initialTitle: "Demo" } });
    await w.find("#ln-title").setValue("Demo");
    await w.findAll("button.mini")[0].trigger("click"); // 加载字段
    await flushPromises();
    expect(api.entryGet).toHaveBeenCalledWith("Demo");
    // 默认勾选 Password；勾选 URL 与自定义字段 api_key
    // （字段顺序：UserName/Password/URL/totp + 自定义 api_key）
    const boxes = w.findAll("input[type=checkbox]");
    await boxes[2].setValue(true); // URL
    await boxes[4].setValue(true); // api_key
    await w.find("#ln-program").setValue("C:/tools/app.exe");
    await w.find("#ln-args").setValue("--verbose");
    await w.find("button.primary").trigger("click");
    await flushPromises();
    expect(api.launchWithEnv).toHaveBeenCalledWith(
      "C:/tools/app.exe",
      ["--verbose"],
      "Demo",
      ["UserName", "Password", "URL", "totp", "api_key"].filter(
        (n) => n === "Password" || n === "URL" || n === "api_key",
      ),
    );
    expect(w.text()).toContain("PID 4321");
    w.unmount();
  });

  it("后端拒绝（缺字段/锁定）时展示错误", async () => {
    api.entryGet.mockResolvedValue(DETAIL);
    api.launchWithEnv.mockRejectedValue(new Error("条目 'Demo' 缺少字段 'api_key'"));
    const w = mount(LaunchDialog, { props: { initialTitle: "Demo" } });
    await w.findAll("button.mini")[0].trigger("click");
    await flushPromises();
    await w.find("#ln-program").setValue("app");
    await w.find("button.primary").trigger("click");
    await flushPromises();
    expect(w.text()).toContain("缺少字段");
    w.unmount();
  });
});
