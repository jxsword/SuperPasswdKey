import { describe, expect, it, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import DetailPanel from "../src/components/DetailPanel.vue";

const api = vi.hoisted(() => ({
  entryGet: vi.fn(),
  entryTotp: vi.fn(),
  clipCopy: vi.fn(),
  entryDelete: vi.fn(),
  entryCreate: vi.fn(),
  entryEdit: vi.fn(),
  attachmentList: vi.fn(),
  attachmentAdd: vi.fn(),
  attachmentDownload: vi.fn(),
  setEditing: vi.fn(),
}));

vi.mock("../src/api", () => ({
  entryGet: api.entryGet,
  entryTotp: api.entryTotp,
  clipCopy: api.clipCopy,
  entryDelete: api.entryDelete,
  entryCreate: api.entryCreate,
  entryEdit: api.entryEdit,
  attachmentList: api.attachmentList,
  attachmentAdd: api.attachmentAdd,
  attachmentDownload: api.attachmentDownload,
  setEditing: api.setEditing,
}));

const baseDetail = {
  title: "GitHub",
  username: "me",
  password: "s3cret",
  url: "https://github.com",
  notes: "some notes",
  otp: null as string | null,
  custom_fields: [{ name: "api_key", value: "k-9", protected: false }],
  tags: ["dev", "infra"],
};

describe("DetailPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.useFakeTimers();
    api.attachmentList.mockResolvedValue(["a.txt", "b.bin"]);
    api.setEditing.mockResolvedValue(undefined);
    api.entryEdit.mockResolvedValue(undefined);
    api.entryCreate.mockResolvedValue(undefined);
  });

  it("编辑保存后（标题未变）详情自动重载", async () => {
    api.entryGet.mockResolvedValue(baseDetail);
    const wrapper = mount(DetailPanel, { props: { title: "GitHub", touch: () => {} } });
    await flushPromises();
    expect(api.entryGet).toHaveBeenCalledTimes(1);
    // 打开编辑表单 → 保存（标题未变）→ onSaved 触发详情重载（entryGet 第二次调用）
    await wrapper.findAll("button.action")[0].trigger("click");
    const form = wrapper.find(".form-panel");
    await form.find("button.primary").trigger("click");
    await flushPromises();
    expect(api.entryGet).toHaveBeenCalledTimes(2);
    wrapper.unmount();
  });

  it("渲染附件列表与下载按钮", async () => {
    api.entryGet.mockResolvedValue(baseDetail);
    api.attachmentDownload.mockResolvedValue("C:\\Downloads\\a.txt");
    const wrapper = mount(DetailPanel, { props: { title: "GitHub", touch: () => {} } });
    await flushPromises();
    expect(wrapper.text()).toContain("a.txt");
    expect(wrapper.text()).toContain("b.bin");
    expect(wrapper.text()).toContain("添加附件");
    await wrapper.findAll("button.copy").filter((b) => b.text() === "下载")[0].trigger("click");
    await flushPromises();
    expect(api.attachmentDownload).toHaveBeenCalledWith("GitHub", "a.txt");
    expect(wrapper.text()).toContain("已保存 a.txt");
    wrapper.unmount();
  });

  it("渲染条目字段与自定义字段（密码掩码）", async () => {
    api.entryGet.mockResolvedValue(baseDetail);
    const wrapper = mount(DetailPanel, { props: { title: "GitHub", touch: () => {} } });
    await flushPromises();
    expect(wrapper.text()).toContain("GitHub");
    expect(wrapper.text()).toContain("me");
    expect(wrapper.text()).toContain("●●●●●●●●");
    expect(wrapper.text()).not.toContain("s3cret"); // 密码不回显明文
    expect(wrapper.text()).toContain("some notes");
    expect(wrapper.text()).toContain("api_key"); // 自定义字段只读展示
    expect(wrapper.text()).toContain("k-9");
    expect(wrapper.text()).toContain("编辑");
    expect(wrapper.text()).toContain("删除");
    wrapper.unmount();
  });

  it("密码下拉：显示切换明文/掩码", async () => {
    api.entryGet.mockResolvedValue(baseDetail);
    const wrapper = mount(DetailPanel, { props: { title: "GitHub", touch: () => {} } });
    await flushPromises();
    await wrapper.find("button.chevron").trigger("click");
    const items = wrapper.findAll("button.menu-item");
    expect(items.map((i) => i.text())).toEqual(["显示", "放大显示", "复制"]);
    await items[0].trigger("click");
    expect(wrapper.text()).toContain("s3cret");
    await wrapper.find("button.chevron").trigger("click");
    await wrapper.findAll("button.menu-item")[0].trigger("click");
    expect(wrapper.text()).not.toContain("s3cret");
    wrapper.unmount();
  });

  it("密码下拉：放大显示打开模态，Esc 关闭", async () => {
    api.entryGet.mockResolvedValue(baseDetail);
    const wrapper = mount(DetailPanel, { props: { title: "GitHub", touch: () => {} } });
    await flushPromises();
    await wrapper.find("button.chevron").trigger("click");
    await wrapper.findAll("button.menu-item")[1].trigger("click");
    const modal = wrapper.find(".large-type-overlay");
    expect(modal.exists()).toBe(true);
    expect(modal.text()).toContain("s3cret");
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    await flushPromises();
    expect(wrapper.find(".large-type-overlay").exists()).toBe(false);
    wrapper.unmount();
  });

  it("有 OTP 时轮询动态码", async () => {
    api.entryGet.mockResolvedValue({ ...baseDetail, otp: "otpauth://totp/x?secret=JBSWY3DPEHPK3PXP" });
    api.entryTotp.mockResolvedValue("123456");
    const wrapper = mount(DetailPanel, { props: { title: "GitHub", touch: () => {} } });
    await flushPromises();
    expect(api.entryTotp).toHaveBeenCalledTimes(1);
    expect(wrapper.text()).toContain("123456");
    vi.advanceTimersByTime(30_000);
    await flushPromises();
    expect(api.entryTotp).toHaveBeenCalledTimes(2);
    wrapper.unmount();
  });

  it("复制按钮调用 clipCopy", async () => {
    api.entryGet.mockResolvedValue(baseDetail);
    api.clipCopy.mockResolvedValue(undefined);
    const wrapper = mount(DetailPanel, { props: { title: "GitHub", touch: () => {} } });
    await flushPromises();
    await wrapper.findAll("button.copy")[0].trigger("click");
    await flushPromises();
    expect(api.clipCopy).toHaveBeenCalledWith("s3cret");
    expect(wrapper.text()).toContain("已复制");
    wrapper.unmount();
  });

  it("删除：确认后调用 entryDelete 并上抛 deleted", async () => {
    api.entryGet.mockResolvedValue(baseDetail);
    api.entryDelete.mockResolvedValue(undefined);
    const wrapper = mount(DetailPanel, { props: { title: "GitHub", touch: () => {} } });
    await flushPromises();
    await wrapper.findAll("button.action")[1].trigger("click"); // 删除按钮
    expect(wrapper.find(".confirm-box").exists()).toBe(true);
    expect(api.entryDelete).not.toHaveBeenCalled();
    await wrapper.find("button.primary.danger").trigger("click");
    await flushPromises();
    expect(api.entryDelete).toHaveBeenCalledWith("GitHub");
    expect(wrapper.emitted("deleted")).toBeTruthy();
    wrapper.unmount();
  });

  it("编辑按钮打开表单模态（预填标题）", async () => {
    api.entryGet.mockResolvedValue(baseDetail);
    const wrapper = mount(DetailPanel, { props: { title: "GitHub", touch: () => {} } });
    await flushPromises();
    await wrapper.findAll("button.action")[0].trigger("click"); // 编辑按钮
    const form = wrapper.find(".form-panel");
    expect(form.exists()).toBe(true);
    expect(form.text()).toContain("编辑：GitHub");
    expect((wrapper.find("#f-title").element as HTMLInputElement).value).toBe("GitHub");
    wrapper.unmount();
  });

  it("锁定语义错误展示", async () => {
    api.entryGet.mockRejectedValue(new Error("会话已锁定，请重新解锁"));
    const wrapper = mount(DetailPanel, { props: { title: "GitHub", touch: () => {} } });
    await flushPromises();
    expect(wrapper.text()).toContain("会话已锁定");
    wrapper.unmount();
  });
});
