/** Cosmetics, loadout, outfits, store catalogue and purchases (Rustic Cloud). */
import { toast } from '@rustic/ui';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { cloud } from '@/cloud/client';
import type { Cosmetic, Currency, Loadout, Outfit, PurchaseRequest } from '@/cloud/types';

export const stk = {
  catalog: (currency: Currency) => ['store', 'catalog', currency] as const,
  product: (id: string, currency: Currency) => ['store', 'product', id, currency] as const,
  cosmetics: ['locker', 'cosmetics'] as const,
  loadout: ['locker', 'loadout'] as const,
  outfits: ['locker', 'outfits'] as const,
  parental: ['parental'] as const,
};

export const useCatalog = (currency: Currency) =>
  useQuery({
    queryKey: stk.catalog(currency),
    queryFn: async () => (await cloud()).catalog(currency),
  });

export const useProduct = (id: string, currency: Currency) =>
  useQuery({
    queryKey: stk.product(id, currency),
    queryFn: async () => (await cloud()).product(id, currency),
  });

export const useCosmetics = () =>
  useQuery({ queryKey: stk.cosmetics, queryFn: async () => (await cloud()).cosmetics() });

export const useLoadout = () =>
  useQuery({ queryKey: stk.loadout, queryFn: async () => (await cloud()).loadout() });

export const useOutfits = () =>
  useQuery({ queryKey: stk.outfits, queryFn: async () => (await cloud()).outfits() });

export const useParental = () =>
  useQuery({ queryKey: stk.parental, queryFn: async () => (await cloud()).parental() });

/** Instant equip: the loadout updates optimistically, rolled back on error. */
export function useEquip() {
  const qc = useQueryClient();
  const { t } = useTranslation('locker');
  return useMutation({
    mutationFn: async (loadout: Loadout) => (await cloud()).equip(loadout),
    onMutate: async (loadout) => {
      await qc.cancelQueries({ queryKey: stk.loadout });
      const previous = qc.getQueryData<Loadout>(stk.loadout);
      qc.setQueryData(stk.loadout, loadout);
      return { previous };
    },
    onError: (_e, _v, ctx) => {
      if (ctx?.previous) qc.setQueryData(stk.loadout, ctx.previous);
      toast({ tone: 'error', title: t('toast.equipFailed') });
    },
    onSuccess: (l) => qc.setQueryData(stk.loadout, l),
  });
}

export function useOutfitActions() {
  const qc = useQueryClient();
  return {
    save: useMutation({
      mutationFn: async (v: { name: string; loadout: Loadout }) =>
        (await cloud()).saveOutfit(v.name, v.loadout),
      onSuccess: (o) => qc.setQueryData<Outfit[]>(stk.outfits, (old) => [...(old ?? []), o]),
    }),
    remove: useMutation({
      mutationFn: async (id: string) => (await cloud()).deleteOutfit(id),
      onMutate: (id) =>
        qc.setQueryData<Outfit[]>(stk.outfits, (old) => old?.filter((o) => o.id !== id)),
      onSettled: () => qc.invalidateQueries({ queryKey: stk.outfits }),
    }),
  };
}

export function usePurchase() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (req: PurchaseRequest) => (await cloud()).purchase(req),
    onSuccess: (res) => {
      qc.setQueryData<Cosmetic[]>(stk.cosmetics, (old) =>
        old?.map((c) =>
          res.granted_cosmetic_ids.includes(c.id)
            ? { ...c, owned: true, acquired_at: new Date().toISOString(), source: 'purchase' }
            : c,
        ),
      );
      void qc.invalidateQueries({ queryKey: ['store'] });
      void qc.invalidateQueries({ queryKey: stk.cosmetics });
    },
  });
}
