import * as DialogPrimitive from "@radix-ui/react-dialog";
import {
  Columns3,
  CornerDownLeft,
  CornerDownRight,
  CornerUpLeft,
  CornerUpRight,
  Rows3,
  Settings2,
  X,
} from "lucide-react";
import { useTranslation } from "react-i18next";

import { Switch } from "@/components/ui/Switch";
import { cn } from "@/lib/utils";
import { useAppStore } from "@/stores/appStore";
import type { OverlayOrientation, OverlayPosition, OverlaySettings } from "@/types";

/** Boolean metric toggles, typed so each row stays checked against the type. */
type MetricKey = "showFps" | "showCpu" | "showGpu" | "showRam" | "showNetwork";

const METRICS: { key: MetricKey; labelKey: string }[] = [
  { key: "showFps", labelKey: "settings.overlayShowFps" },
  { key: "showCpu", labelKey: "settings.overlayShowCpu" },
  { key: "showGpu", labelKey: "settings.overlayShowGpu" },
  { key: "showRam", labelKey: "settings.overlayShowRam" },
  { key: "showNetwork", labelKey: "settings.overlayShowNetwork" },
];

const POSITIONS: { value: OverlayPosition; icon: typeof CornerUpLeft; labelKey: string }[] = [
  { value: "top-left", icon: CornerUpLeft, labelKey: "settings.overlayPosTopLeft" },
  { value: "top-right", icon: CornerUpRight, labelKey: "settings.overlayPosTopRight" },
  { value: "bottom-left", icon: CornerDownLeft, labelKey: "settings.overlayPosBottomLeft" },
  { value: "bottom-right", icon: CornerDownRight, labelKey: "settings.overlayPosBottomRight" },
];

const ORIENTATIONS: {
  value: OverlayOrientation;
  icon: typeof Rows3;
  labelKey: string;
}[] = [
  { value: "vertical", icon: Rows3, labelKey: "settings.overlayOrientationVertical" },
  { value: "horizontal", icon: Columns3, labelKey: "settings.overlayOrientationHorizontal" },
];

export function OverlayConfigDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useTranslation();
  const overlay = useAppStore((state) => state.settings.overlay);
  const updateSettings = useAppStore((state) => state.updateSettings);

  // Each control saves immediately: the overlay is visible next to the app, so
  // its effect on the panel is immediate feedback.
  const patch = (next: Partial<OverlaySettings>) => {
    void updateSettings({ overlay: { ...overlay, ...next } });
  };

  return (
    <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
      <DialogPrimitive.Portal>
        <DialogPrimitive.Overlay className="ovdlg-overlay" />
        <DialogPrimitive.Content className="ovdlg">
          <div className="panel-head">
            <DialogPrimitive.Title asChild>
              <h3 className="panel-title">
                <Settings2 size={14} strokeWidth={2} aria-hidden />
                {t("settings.overlayDialogTitle")}
              </h3>
            </DialogPrimitive.Title>
            <DialogPrimitive.Close asChild>
              <button
                type="button"
                className="btn btn-ghost btn-sm"
                aria-label={t("settings.overlayClose")}
              >
                <X size={13} aria-hidden />
              </button>
            </DialogPrimitive.Close>
          </div>

          <DialogPrimitive.Description className="panel-sub">
            {t("settings.overlayDialogHint")}
          </DialogPrimitive.Description>

          <div className="ovdlg-section">
            <h4>{t("settings.overlayMetrics")}</h4>
            {METRICS.map((metric) => (
              <div className="row" key={metric.key}>
                <div className="row-label">
                  <b>{t(metric.labelKey)}</b>
                </div>
                <div style={{ flexShrink: 0 }}>
                  <Switch
                    checked={overlay[metric.key]}
                    onCheckedChange={(checked) =>
                      patch({ [metric.key]: checked } as Partial<OverlaySettings>)
                    }
                  />
                </div>
              </div>
            ))}
          </div>

          <div className="ovdlg-section">
            <h4>{t("settings.overlayPosition")}</h4>
            <div className="ovpos-grid">
              {POSITIONS.map((position) => {
                const Icon = position.icon;
                return (
                  <button
                    key={position.value}
                    type="button"
                    onClick={() => patch({ position: position.value })}
                    className={cn("ovpos", overlay.position === position.value && "active")}
                    aria-pressed={overlay.position === position.value}
                  >
                    <Icon size={14} aria-hidden />
                    {t(position.labelKey)}
                  </button>
                );
              })}
            </div>
          </div>

          <div className="ovdlg-section">
            <h4>{t("settings.overlayOrientation")}</h4>
            <div className="ovpos-grid">
              {ORIENTATIONS.map((orientation) => {
                const Icon = orientation.icon;
                return (
                  <button
                    key={orientation.value}
                    type="button"
                    onClick={() => patch({ orientation: orientation.value })}
                    className={cn("ovpos", overlay.orientation === orientation.value && "active")}
                    aria-pressed={overlay.orientation === orientation.value}
                  >
                    <Icon size={14} aria-hidden />
                    {t(orientation.labelKey)}
                  </button>
                );
              })}
            </div>
          </div>

          <div className="ovdlg-section">
            <h4>{t("settings.overlayOpacity")}</h4>
            <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
              <input
                type="range"
                min={35}
                max={100}
                step={1}
                className="range"
                value={Math.round(overlay.opacity * 100)}
                onChange={(event) => patch({ opacity: Number(event.target.value) / 100 })}
                aria-label={t("settings.overlayOpacity")}
              />
              <span className="ovpct">{Math.round(overlay.opacity * 100)} %</span>
            </div>
          </div>
        </DialogPrimitive.Content>
      </DialogPrimitive.Portal>
    </DialogPrimitive.Root>
  );
}
