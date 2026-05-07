export const KNOWN_DOFUS_CLASSES = [
  "cra",
  "ecaflip",
  "eliotrope",
  "eniripsa",
  "enutrof",
  "feca",
  "forgelance",
  "huppermage",
  "iop",
  "osamodas",
  "ouginak",
  "pandawa",
  "roublard",
  "sacrieur",
  "sadida",
  "sram",
  "steamer",
  "xelor",
  "zobal",
] as const;

const CLASS_SET = new Set<string>(KNOWN_DOFUS_CLASSES);

export function avatarUrlFor(dofusClass: string | null | undefined): string | null {
  if (!dofusClass) return null;
  return CLASS_SET.has(dofusClass) ? `/avatars/${dofusClass}.jpg` : null;
}

const DISPLAY_NAMES: Record<string, string> = {
  cra: "Crâ",
  ecaflip: "Ecaflip",
  eliotrope: "Eliotrope",
  eniripsa: "Eniripsa",
  enutrof: "Enutrof",
  feca: "Féca",
  forgelance: "Forgelance",
  huppermage: "Huppermage",
  iop: "Iop",
  osamodas: "Osamodas",
  ouginak: "Ouginak",
  pandawa: "Pandawa",
  roublard: "Roublard",
  sacrieur: "Sacrieur",
  sadida: "Sadida",
  sram: "Sram",
  steamer: "Steamer",
  xelor: "Xelor",
  zobal: "Zobal",
};

export function classDisplayName(dofusClass: string | null | undefined): string | null {
  if (!dofusClass) return null;
  return DISPLAY_NAMES[dofusClass] ?? dofusClass;
}
