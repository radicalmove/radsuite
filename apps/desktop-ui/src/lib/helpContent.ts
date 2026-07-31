export type HelpSection = {
  id: string;
  title: string;
  summary: string;
  steps: string[];
};

export type HelpFaq = {
  question: string;
  answer: string;
};

export const helpSections: HelpSection[] = [
  {
    id: "getting-started",
    title: "Start with a project",
    summary: "A project represents one course and keeps its documents, readings, references, and exports together.",
    steps: [
      "Choose an existing course in the left sidebar, or create a new project.",
      "Open Documents, choose a DOCX or PDF, and select Analyse.",
      "Open the saved review to inspect paragraphs and citation flags.",
    ],
  },
  {
    id: "citation-review",
    title: "Review citations",
    summary: "Select a paragraph to see its citation actions and keep the review decisions with the document.",
    steps: [
      "Select a paragraph to see detected citations and paragraphs that may need sources.",
      "Use Search sources to find Crossref and OpenAlex matches, or Copy search query to paste the search into another library service.",
      "Use Verify citations, Mark reviewed manually, or Not required to record the outcome.",
    ],
  },
  {
    id: "readings-and-exports",
    title: "Build readings and exports",
    summary: "The same analysed document can feed the course reading list, while references and readings can be exported when ready.",
    steps: [
      "Choose Use for readings beside a saved DOCX review, or open Readings to import DOCX, PDF, or CSV sources.",
      "Review required and optional readings, assign modules, and use Find sources to complete missing details.",
      "Open Exports to create and copy or download course-reference and module-reading HTML.",
    ],
  },
];

export const helpFaqs: HelpFaq[] = [
  {
    question: "Where is my work saved?",
    answer: "RADsuite saves your work on this Mac and keeps it available offline. The Saved on this Mac status confirms local saving is available.",
  },
  {
    question: "What does cloud sync mean?",
    answer: "Cloud sync is optional. When it is not connected, your work is still saved on this Mac but is not copied to another device.",
  },
  {
    question: "How do I reuse an analysed document?",
    answer: "Open Documents and choose Use for readings beside a saved DOCX review. RADsuite keeps a local copy of analysed sources so the readings workflow can reuse it after a restart.",
  },
  {
    question: "How do I restore something I archived?",
    answer: "Open Archive under the current project, find the document, module, reading, or reference, and choose Restore.",
  },
];
