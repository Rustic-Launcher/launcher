/** Client-side checks of the Rustic sign-up form (the cloud re-checks everything). */
export interface SignupForm {
  email: string;
  password: string;
  username: string;
  date_of_birth: string;
  country: string;
  accept_privacy: boolean;
}

export type SignupProblem =
  'email' | 'password' | 'username' | 'dateOfBirth' | 'tooYoung' | 'privacy';

/** Minimum account age (AGENTS.md user decisions). */
export const MIN_ACCOUNT_AGE = 13;

export function ageOn(dateOfBirth: string, now: Date): number | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(dateOfBirth);
  if (!m) return null;
  const [y, mo, d] = [Number(m[1]), Number(m[2]), Number(m[3])];
  let age = now.getFullYear() - y;
  if (now.getMonth() + 1 < mo || (now.getMonth() + 1 === mo && now.getDate() < d)) age--;
  return age;
}

export function validateSignup(f: SignupForm, now = new Date()): SignupProblem[] {
  const out: SignupProblem[] = [];
  if (!/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(f.email.trim())) out.push('email');
  if (f.password.length < 10) out.push('password');
  if (!/^[A-Za-z0-9_]{3,20}$/.test(f.username.trim())) out.push('username');
  const age = ageOn(f.date_of_birth, now);
  if (age === null || age > 120) out.push('dateOfBirth');
  else if (age < MIN_ACCOUNT_AGE) out.push('tooYoung');
  if (!f.accept_privacy) out.push('privacy');
  return out;
}
