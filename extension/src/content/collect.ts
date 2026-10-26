// DOM 适配层（design_docs/12 §2）：文档 → 描述符列表。薄壳，不进单测。
import type { InputDescriptor } from "./detect";

function isVisible(el: HTMLInputElement): boolean {
  const style = window.getComputedStyle(el);
  if (style.display === "none" || style.visibility === "hidden") return false;
  const rect = el.getBoundingClientRect();
  return rect.width > 0 && rect.height > 0;
}

const FORM_TYPES = new Set([
  "checkbox",
  "radio",
  "submit",
  "button",
  "image",
  "file",
  "reset",
]);

export function collectInputs(): InputDescriptor[] {
  const formIds = new Map<HTMLFormElement, number>();
  let nextFormSeq = 0;
  const out: InputDescriptor[] = [];
  const inputs = Array.from(
    document.querySelectorAll("input"),
  ) as HTMLInputElement[];
  inputs.forEach((el, index) => {
    let type = (el.getAttribute("type") ?? "text").toLowerCase();
    if (FORM_TYPES.has(type)) return; // 非文本语义控件不参与（规则 2）
    if (!type) type = "text";
    const form = el.closest("form");
    let formSeq = -1;
    if (form) {
      let seq = formIds.get(form);
      if (seq === undefined) {
        seq = nextFormSeq++;
        formIds.set(form, seq);
      }
      formSeq = seq;
    }
    out.push({
      index,
      type,
      name: el.name ?? "",
      id: el.id ?? "",
      autocomplete: (el.getAttribute("autocomplete") ?? "").toLowerCase(),
      visible: isVisible(el),
      formSeq,
    });
  });
  return out;
}

export function inputAt(index: number): HTMLInputElement | null {
  const inputs = Array.from(
    document.querySelectorAll("input"),
  ) as HTMLInputElement[];
  return inputs[index] ?? null;
}
