/** Framework-independent contracts for the Folio Penpot design system. */
import type { IconName } from './icons/icon-names';
export type IntentStatus = 'open' | 'completed' | 'priority';
export type ControlState = 'default' | 'hover' | 'focus' | 'disabled';

export interface ButtonProps {
  label: string;
  appearance?: 'primary' | 'secondary' | 'ghost';
  size?: 'sm' | 'md';
  icon?: IconName;
  iconPosition?: 'leading' | 'trailing';
  disabled?: boolean;
  onPress: () => void;
}
export interface IconButtonProps {
  icon: IconName;
  ariaLabel: string;
  disabled?: boolean;
  onPress: () => void;
}
export interface TaskRowProps {
  id: string;
  title: string;
  status: IntentStatus;
  timeLabel?: string;
  badge?: string;
  onStatusChange: (id: string, next: IntentStatus) => void;
  onOpen?: (id: string) => void;
}
export interface GoalCardProps {
  id: string;
  eyebrow: string;
  title: string;
  description: string;
  statusLabel: string;
  cadenceLabel?: string;
  onOpen?: (id: string) => void;
}
export interface NoteCardProps {
  id: string;
  title: string;
  excerpt: string;
  folderLabel: string;
  updatedLabel: string;
  onOpen: (id: string) => void;
}
export interface PlannerEventProps {
  id: string;
  title: string;
  timeLabel?: string;
  status: IntentStatus;
}
export interface PlannerDayProps {
  date: string;
  dayLabel: string;
  dayNumber: number;
  isToday?: boolean;
  isWeekend?: boolean;
  events: readonly PlannerEventProps[];
  onAdd: (date: string) => void;
  onEventOpen: (id: string) => void;
}
export interface NavigationItemProps {
  id: string;
  label: string;
  icon: IconName;
  href: string;
  selected?: boolean;
  depth?: number;
}
export interface SearchFieldProps {
  value: string;
  placeholder?: string;
  shortcut?: string;
  disabled?: boolean;
  onChange: (value: string) => void;
}

/** Theme is inherited from the application/token scope, not passed to every card. */
export type FolioTheme = 'light' | 'dark';
