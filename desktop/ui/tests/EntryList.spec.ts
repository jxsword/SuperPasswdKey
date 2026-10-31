import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import EntryList from "../src/components/EntryList.vue";
import type { EntrySummary } from "../src/api";

const entries: EntrySummary[] = [
  { title: "GitHub", username: "me", url: "https://github.com", has_otp: true, tags: ["dev"] },
  { title: "Local", username: null, url: null, has_otp: false, tags: [] },
];

describe("EntryList", () => {
  it("渲染条目标题/用户名/TOTP 标记", () => {
    const wrapper = mount(EntryList, {
      props: { entries, selected: null, loading: false },
    });
    expect(wrapper.text()).toContain("GitHub");
    expect(wrapper.text()).toContain("me");
    expect(wrapper.text()).toContain("TOTP");
    expect(wrapper.text()).toContain("Local");
  });

  it("点击条目发出 select 事件", async () => {
    const wrapper = mount(EntryList, {
      props: { entries, selected: null, loading: false },
    });
    await wrapper.findAll("button.entry-item")[0].trigger("click");
    expect(wrapper.emitted("select")?.[0]).toEqual(["GitHub"]);
  });

  it("选中条目高亮", () => {
    const wrapper = mount(EntryList, {
      props: { entries, selected: "GitHub", loading: false },
    });
    expect(wrapper.findAll("button.entry-item")[0].classes()).toContain("selected");
    expect(wrapper.findAll("button.entry-item")[1].classes()).not.toContain("selected");
  });

  it("空列表显示提示", () => {
    const wrapper = mount(EntryList, {
      props: { entries: [], selected: null, loading: false },
    });
    expect(wrapper.text()).toContain("无匹配条目");
  });

  it("点击标签徽标发出 tag 事件（按标签过滤入口）", async () => {
    const wrapper = mount(EntryList, {
      props: { entries, selected: null, loading: false },
    });
    await wrapper.find("button.tag-badge").trigger("click");
    expect(wrapper.emitted("tag")?.[0]).toEqual(["dev"]);
  });

  it("URL 提取 hostname", () => {
    const wrapper = mount(EntryList, {
      props: {
        entries: [{ title: "X", username: null, url: "https://example.com/path", has_otp: false, tags: [] }],
        selected: null,
        loading: false,
      },
    });
    expect(wrapper.text()).toContain("example.com");
  });
});
