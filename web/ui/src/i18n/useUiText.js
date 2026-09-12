import { useCallback } from 'react';
import { useI18n } from './I18nContext';
import { translateUiText } from './catalog.js';

export default function useUiText() {
  const { locale } = useI18n({ optional: true });
  return useCallback((source, params = null) => translateUiText(source, params, locale), [locale]);
}
