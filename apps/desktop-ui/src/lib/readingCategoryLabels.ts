export function readingCategoryLabel(category: string): string {
  if (category === "compulsory") {
    return "Required";
  }

  if (category === "optional") {
    return "Optional";
  }

  return category;
}
