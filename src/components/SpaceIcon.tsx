import { Bookmark, BookOpen, Briefcase, Code2, Folder, Heart, Lightbulb, Presentation, Star, type LucideIcon } from "lucide-react";

export const SPACE_ICONS: Record<string, LucideIcon> = {
  folder: Folder,
  code: Code2,
  bookmark: Bookmark,
  read: BookOpen,
  work: Briefcase,
  star: Star,
  heart: Heart,
  idea: Lightbulb,
  slides: Presentation,
};

export const SPACE_COLORS = ["#6366F1", "#10B981", "#F59E0B", "#EF4444", "#EC4899", "#8B5CF6", "#0EA5E9", "#64748B"];

export function SpaceIcon({ name, size = 14 }: { name: string | null; size?: number }) {
  const Icon = SPACE_ICONS[name ?? "folder"] ?? Folder;
  return <Icon size={size} strokeWidth={2} />;
}
