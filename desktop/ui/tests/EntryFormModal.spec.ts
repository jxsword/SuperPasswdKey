import { describe, expect, it, vi, beforeEach } from "vitest";
import { mount, flushPromises } from "@vue/test-utils";
import EntryFormModal from "../src/components/EntryFormModal.vue";

const api = vi.hoisted(() => ({
  entryCreate: vi.fn(),
  entryEdit: vi.fn(),
  setEditing: vi.fn(),
}));

vi.mock("../src/api", () => ({
  entryCreate: api.entryCreate,
  entryEdit: api.entryEdit,
  setEditing: api.setEditing,
}));

const initial = {
  title: "GitHub",
  username: "me",
  password: "s3cret",
  url: "https://github.com",
  notes: null,
  otp: null,
  custom_fields: [{ name: "api_key", value: "k-9", protected: false }],
  tags: ["dev"],
};

describe("EntryFormModal", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    api.setEditing.mockResolvedValue(undefined);
  });

  it("挂载/卸载切换编辑态（暂停空闲锁定）", async () => {
    const wrapper = mount(EntryFormModal, { props: { mode: "create" } });
    await flushPromises();
    expect(api.setEditing).toHaveBeenCalledWith(true);
    wrapper.unmount();
    expect(api.setEditing).toHaveBeenLastCalledWith(false);
  });

  it("创建模式：空标题报错不提交", async () => {
    const wrapper = mount(EntryFormModal, { props: { mode: "create" } });
    await wrapper.find("button.primary").trigger("click");
    expect(api.entryCreate).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain("标题必填");
    wrapper.unmount();
  });

  it("创建模式：otp 非 otpauth:// 报错（对齐 CLI 退出码 4 语义）", async () => {
    const wrapper = mount(EntryFormModal, { props: { mode: "create" } });
    await wrapper.find("#f-title").setValue("X");
    await wrapper.find("#f-otp").setValue("not-otpauth");
    await wrapper.find("button.primary").trigger("click");
    expect(api.entryCreate).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain("otpauth://");
    wrapper.unmount();
  });

  it("创建模式：提交调用 entryCreate 并上抛 saved(null, title)", async () => {
    api.entryCreate.mockResolvedValue(undefined);
    const wrapper = mount(EntryFormModal, { props: { mode: "create" } });
    await wrapper.find("#f-title").setValue(" New Entry ");
    await wrapper.find("#f-password").setValue("p1");
    await wrapper.find("#f-tags").setValue(" work, infra ， 生产 ");
    await wrapper.find("button.primary").trigger("click");
    await flushPromises();
    expect(api.entryCreate).toHaveBeenCalledWith(
      expect.objectContaining({
        title: "New Entry",
        password: "p1",
        custom_fields: [],
        tags: ["work", "infra", "生产"], // 中英文逗号分隔 + trim
      }),
    );
    expect(wrapper.emitted("saved")?.[0]).toEqual([null, "New Entry"]);
    wrapper.unmount();
  });

  it("编辑模式：预填 + custom_fields 原样回传（保真口径）", async () => {
    api.entryEdit.mockResolvedValue(undefined);
    const wrapper = mount(EntryFormModal, { props: { mode: "edit", initial } });
    expect((wrapper.find("#f-title").element as HTMLInputElement).value).toBe("GitHub");
    expect((wrapper.find("#f-password").element as HTMLInputElement).value).toBe("s3cret");
    // 标题改名 + 密码修改，custom_fields 未渲染编辑控件但随草稿回传
    await wrapper.find("#f-title").setValue("GitHub2");
    await wrapper.find("#f-password").setValue("p2");
    await wrapper.find("button.primary").trigger("click");
    await flushPromises();
    expect(api.entryEdit).toHaveBeenCalledWith(
      "GitHub",
      expect.objectContaining({
        title: "GitHub2",
        password: "p2",
        custom_fields: initial.custom_fields,
      }),
    );
    expect(wrapper.emitted("saved")?.[0]).toEqual(["GitHub", "GitHub2"]);
    wrapper.unmount();
  });

  it("编辑模式：自定义字段行预填可编辑（S1b-2b）", async () => {
    api.entryEdit.mockResolvedValue(undefined);
    const wrapper = mount(EntryFormModal, { props: { mode: "edit", initial } });
    const names = wrapper.findAll("input.cf-name");
    expect(names).toHaveLength(1);
    expect((names[0].element as HTMLInputElement).value).toBe("api_key");
    // 保留名校验
    await names[0].setValue("Password");
    await wrapper.find("button.primary").trigger("click");
    expect(api.entryEdit).not.toHaveBeenCalled();
    expect(wrapper.text()).toContain("保留名");
    // 改回并提交：custom_fields 随草稿回传
    await names[0].setValue("api_key2");
    await wrapper.find("#f-title").setValue("GitHub");
    await wrapper.find("button.primary").trigger("click");
    await flushPromises();
    expect(api.entryEdit).toHaveBeenCalledWith(
      "GitHub",
      expect.objectContaining({
        custom_fields: [{ name: "api_key2", value: "k-9", protected: false }],
      }),
    );
    wrapper.unmount();
  });

  it("添加/删除自定义字段行", async () => {
    const wrapper = mount(EntryFormModal, { props: { mode: "create" } });
    expect(wrapper.findAll("input.cf-name")).toHaveLength(0);
    await wrapper.find("button.cf-add").trigger("click");
    await wrapper.find("button.cf-add").trigger("click");
    expect(wrapper.findAll("input.cf-name")).toHaveLength(2);
    const rows = wrapper.findAll(".cf-row");
    await rows[0].findAll("button")[0].trigger("click"); // ✕ 删除第一行
    expect(wrapper.findAll("input.cf-name")).toHaveLength(1);
    // 空名行提交时被过滤（名称与值均空的行不进草稿）
    await wrapper.find("#f-title").setValue("T");
    await wrapper.find("button.primary").trigger("click");
    await flushPromises();
    expect(api.entryCreate).toHaveBeenCalledWith(
      expect.objectContaining({ custom_fields: [] }),
    );
    wrapper.unmount();
  });
});
