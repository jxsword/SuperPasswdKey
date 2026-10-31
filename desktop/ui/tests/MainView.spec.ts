import { describe, expect, it, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import MainView from "../src/components/MainView.vue";
import type { EntrySummary } from "../src/api";

const api = vi.hoisted(() => ({
  entryList: vi.fn(),
  entryGet: vi.fn(),
  entryTotp: vi.fn(),
  clipCopy: vi.fn(),
  entryCreate: vi.fn(),
  entryEdit: vi.fn(),
  entryDelete: vi.fn(),
}));

vi.mock("../src/api", () => ({
  entryList: api.entryList,
  entryGet: api.entryGet,
  entryTotp: api.entryTotp,
  clipCopy: api.clipCopy,
  entryCreate: api.entryCreate,
  entryEdit: api.entryEdit,
  entryDelete: api.entryDelete,
}));

const entries: EntrySummary[] = [
  { title: "GitHub", username: "me", url: "https://github.com", has_otp: false },
  { title: "AWS Console", username: "admin", url: "https://aws.amazon.com", has_otp: true },
  { title: "Local App", username: null, url: null, has_otp: false },
];

describe("MainView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.entryList.mockResolvedValue(entries);
  });

  it("挂载时加载条目列表", async () => {
    const wrapper = mount(MainView, { props: { touch: () => {} } });
    await flushPromises();
    expect(api.entryList).toHaveBeenCalledTimes(1);
    expect(wrapper.text()).toContain("GitHub");
    expect(wrapper.text()).toContain("AWS Console");
  });

  it("搜索按标题/用户名过滤", async () => {
    const wrapper = mount(MainView, { props: { touch: () => {} } });
    await flushPromises();
    const input = wrapper.find('input[type="search"]');
    await input.setValue("github");
    expect(wrapper.text()).toContain("GitHub");
    expect(wrapper.text()).not.toContain("AWS Console");
    await input.setValue("admin");
    expect(wrapper.text()).toContain("AWS Console");
    expect(wrapper.text()).not.toContain("GitHub");
    await input.setValue("zzzz");
    expect(wrapper.text()).toContain("无匹配条目");
  });

  it("点击条目加载详情", async () => {
    api.entryGet.mockResolvedValue({
      title: "GitHub",
      username: "me",
      password: "secret",
      url: "https://github.com",
      notes: null,
      otp: null,
      custom_fields: [],
    });
    const wrapper = mount(MainView, { props: { touch: () => {} } });
    await flushPromises();
    await wrapper.findAll("button.entry-item")[0].trigger("click");
    await flushPromises();
    expect(api.entryGet).toHaveBeenCalledWith("GitHub");
    expect(wrapper.text()).toContain("GitHub");
    expect(wrapper.text()).toContain("●●●●●●●●");
  });

  it("「新的项目」→ 创建成功后刷新列表并选中新条目", async () => {
    const wrapper = mount(MainView, { props: { touch: () => {} } });
    await flushPromises();
    await wrapper.find("button.new-entry").trigger("click");
    const modal = wrapper.find(".form-panel");
    expect(modal.exists()).toBe(true);
    expect(modal.text()).toContain("新的项目");
    api.entryCreate.mockResolvedValue(undefined);
    api.entryList.mockResolvedValue([
      ...entries,
      { title: "Brand New", username: null, url: null, has_otp: false },
    ]);
    await modal.find("#f-title").setValue("Brand New");
    await modal.find("button.primary").trigger("click");
    await flushPromises();
    expect(api.entryCreate).toHaveBeenCalled();
    expect(wrapper.find(".form-panel").exists()).toBe(false);
    // 列表刷新且新条目选中（详情加载）
    expect(wrapper.text()).toContain("Brand New");
    expect(api.entryGet).toHaveBeenCalledWith("Brand New");
    wrapper.unmount();
  });

  it("删除后清空详情并刷新列表", async () => {
    api.entryGet.mockResolvedValue({
      title: "GitHub",
      username: "me",
      password: "secret",
      url: null,
      notes: null,
      otp: null,
      custom_fields: [],
    });
    api.entryDelete.mockResolvedValue(undefined);
    const wrapper = mount(MainView, { props: { touch: () => {} } });
    await flushPromises();
    await wrapper.findAll("button.entry-item")[0].trigger("click");
    await flushPromises();
    await wrapper.findAll("button.action")[1].trigger("click"); // 删除
    await wrapper.find("button.primary.danger").trigger("click");
    await flushPromises();
    expect(api.entryDelete).toHaveBeenCalledWith("GitHub");
    expect(wrapper.text()).toContain("从左侧选择一个条目查看详情");
    wrapper.unmount();
  });
});
