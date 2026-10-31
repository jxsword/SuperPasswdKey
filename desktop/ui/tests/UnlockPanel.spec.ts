import { describe, expect, it, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import UnlockPanel from "../src/components/UnlockPanel.vue";

const api = vi.hoisted(() => ({
  sessionStatus: vi.fn(),
  unlock: vi.fn(),
  touch: vi.fn(),
  getSettings: vi.fn(),
  keyfilePick: vi.fn(),
}));

vi.mock("../src/api", () => ({
  sessionStatus: api.sessionStatus,
  unlock: api.unlock,
  touch: api.touch,
  getSettings: api.getSettings,
  keyfilePick: api.keyfilePick,
}));

describe("UnlockPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.getSettings.mockResolvedValue({
      idle_ms: 300_000,
      absolute_ms: 1_800_000,
      autostart: false,
      keyfile_path: "",
    });
    api.sessionStatus.mockResolvedValue({
      unlocked: false,
      idle_remaining_ms: 0,
      absolute_remaining_ms: 0,
      vault_path: "C:\\vault.kdbx",
      vault_exists: true,
    });
  });

  it("keyfile 路径自动预填（上次成功解锁的路径）", async () => {
    api.getSettings.mockResolvedValue({
      idle_ms: 300_000,
      absolute_ms: 1_800_000,
      autostart: false,
      keyfile_path: "C:\\keys\\vault.keyfile",
    });
    const wrapper = mount(UnlockPanel, { props: { error: "" } });
    await flushPromises();
    expect(
      (wrapper.find("input[type=text]").element as HTMLInputElement).value,
    ).toBe("C:\\keys\\vault.keyfile");
    wrapper.unmount();
  });

  it("浏览按钮选择文件填入路径", async () => {
    api.keyfilePick.mockResolvedValue("D:\\keys\\picked.keyfile");
    const wrapper = mount(UnlockPanel, { props: { error: "" } });
    await flushPromises();
    await wrapper.find("button.browse").trigger("click");
    await flushPromises();
    expect(api.keyfilePick).toHaveBeenCalledTimes(1);
    expect(
      (wrapper.find("input[type=text]").element as HTMLInputElement).value,
    ).toBe("D:\\keys\\picked.keyfile");
    wrapper.unmount();
  });

  it("空密码不提交", async () => {
    const wrapper = mount(UnlockPanel, { props: { error: "" } });
    await wrapper.find("button.submit").trigger("click");
    await flushPromises();
    expect(api.unlock).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain("请输入主密码");
  });

  it("提交解锁并发出 unlocked 事件", async () => {
    api.unlock.mockResolvedValue({
      unlocked: true,
      idle_remaining_ms: 300_000,
      absolute_remaining_ms: 1_800_000,
      vault_path: "C:\\vault.kdbx",
      vault_exists: true,
    });
    const wrapper = mount(UnlockPanel, { props: { error: "" } });
    await wrapper.find("input[type=password]").setValue("my-pass");
    await wrapper.find("button.submit").trigger("click");
    await flushPromises();
    expect(api.unlock).toHaveBeenCalledWith("my-pass", null);
    expect(wrapper.emitted("unlocked")).toBeTruthy();
    wrapper.unmount();
  });

  it("解锁失败展示错误", async () => {
    api.unlock.mockRejectedValue(new Error("解锁失败：主密码或 keyfile 不正确"));
    const wrapper = mount(UnlockPanel, { props: { error: "" } });
    await wrapper.find("input[type=password]").setValue("wrong");
    await wrapper.find("button.submit").trigger("click");
    await flushPromises();
    expect(wrapper.text()).toContain("解锁失败");
    wrapper.unmount();
  });
});
