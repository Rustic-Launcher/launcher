import '@testing-library/jest-dom/vitest';
import './i18n';

// The mock backend answers instantly in unit tests.
window.localStorage.setItem('rustic.mock.latency', '0');
