export interface PanelSettingsForm {
  panelName: string
  enableRegister: boolean
  steamApiKey: string
}

export function normalizePanelSettings(form: PanelSettingsForm): PanelSettingsForm {
  return {
    panelName: form.panelName.trim(),
    enableRegister: form.enableRegister,
    steamApiKey: form.steamApiKey.trim(),
  }
}
