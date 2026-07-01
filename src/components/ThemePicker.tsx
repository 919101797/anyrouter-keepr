import { Palette } from "lucide-react";
import { APP_THEME_OPTIONS, appThemeLabel, type AppTheme } from "../lib/appTheme";
import { Select, SelectContent, SelectItem, SelectTrigger } from "./ui/select";
import { Tooltip, TooltipContent, TooltipTrigger } from "./ui/tooltip";

interface ThemePickerProps {
  theme: AppTheme;
  onThemeChange: (theme: AppTheme) => void;
}

export function ThemePicker({ theme, onThemeChange }: ThemePickerProps) {
  return (
    <Select value={theme} onValueChange={(value) => onThemeChange(value as AppTheme)}>
      <Tooltip>
        <TooltipTrigger asChild>
          <SelectTrigger className="theme-picker-trigger" aria-label={`主题：${appThemeLabel(theme)}`}>
            <Palette className="h-4 w-4" />
            <span className="theme-picker-current">{appThemeLabel(theme)}</span>
          </SelectTrigger>
        </TooltipTrigger>
        <TooltipContent>主题：{appThemeLabel(theme)}</TooltipContent>
      </Tooltip>
      <SelectContent>
        {APP_THEME_OPTIONS.map((option) => (
          <SelectItem key={option.value} value={option.value}>
            <span className="theme-picker-item">
              <span>{option.label}</span>
              <span>{option.description}</span>
            </span>
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
