import { describe, expect, it, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import MainView from "../src/components/MainView.vue";
import type { EntrySummary } from "../src/api";

const api = vi.hoisted(() => ({
  entryList: vi.fn(),
  entryGet: vi.fn(),
  entryTotp: vi.fn(),
  clipCopy: vi.fn(),
  touch: vi.fn(),
}));

vi.mock("../src/api", () => ({
  entryList: api.entryList,
  entryGet: api.entryGet,
  entryTotp: api.entryTotp,
  clipCopy: api.clipCopy,
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
    });
    const wrapper = mount(MainView, { props: { touch: () => {} } });
    await flushPromises();
    await wrapper.findAll("button.entry-item")[0].trigger("click");
    await flushPromises();
    expect(api.entryGet).toHaveBeenCalledWith("GitHub");
    expect(wrapper.text()).toContain("GitHub");
    expect(wrapper.text()).toContain("●●●●●●●●");
  });
});
