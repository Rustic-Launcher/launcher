//! The trait mod (`horizon_client::mod_api::Hooks`): the client tick that drives the network and
//! caches, the keys that open the social overlay and the emote wheel, the mod's own screen that
//! draws them, a small HUD line for pending invites, discreet toasts, and the title menu
//! ([`crate::title_menu`]).
use crate::net::Job;
use crate::state::{MOD_ID, Mode, OwnEmote, guard, guard_or, lock, net};
use horizon_client::mod_api::{
    ClientChatReceivedEvent, ClientDisconnectingEvent, ClientResourcesReloadedEvent,
    ClientTickPostEvent, Corner, Flow, FrameEvent, Hooks, Host, HudEvent, KeyEvent,
    NativeScreenKind, ScreenClosingEvent, ScreenKeyPressedPreEvent,
    ScreenMouseButtonPressedPreEvent, ScreenRenderPostEvent, Text,
};
use rusticmc_companion_core::overlay::{self, Action, Draw, View};
use rusticmc_companion_core::protocol::WornItem;
use rusticmc_companion_core::{parse_uuid, wheel};
use std::time::Instant;

pub struct Companion;

const BOX: u32 = 0x7800_0000;

fn ours(host: &dyn Host) -> bool {
    host.mod_screen() == Some(MOD_ID)
}

impl Hooks for Companion {
    fn client_tick_post(&self, host: &mut dyn Host, _event: &mut ClientTickPostEvent) {
        guard("client_tick_post", || client_tick(host));
    }

    fn frame(&self, host: &mut dyn Host, _event: &mut FrameEvent) {
        crate::title_menu::frame(host);
    }

    fn client_resources_reloaded(
        &self,
        _host: &mut dyn Host,
        _event: &mut ClientResourcesReloadedEvent,
    ) {
        crate::title_menu::resources_reloaded();
    }

    fn key(&self, host: &mut dyn Host, event: &mut KeyEvent<'_>) {
        guard("key", || {
            if !event.pressed {
                return;
            }
            let playing = host
                .client_state()
                .is_some_and(|c| c.screen == NativeScreenKind::Playing);
            let open = ours(host);
            let (want, current) = {
                let st = lock();
                let want = if event.key == st.cfg.overlay_key {
                    Mode::Overlay
                } else if event.key == st.cfg.emote_key {
                    Mode::Wheel
                } else {
                    return;
                };
                (want, st.ui.mode)
            };
            // No lock held while the game opens or closes a screen (it may run hooks).
            if open && current == Some(want) {
                host.mod_screen_close(MOD_ID);
                lock().ui.mode = None;
            } else if playing && !open && host.mod_screen_open(MOD_ID) {
                let mut st = lock();
                st.ui.mode = Some(want);
                if want == Mode::Overlay {
                    st.social.refresh_soon();
                }
            }
        });
    }

    fn screen_render_post(&self, host: &dyn Host, event: &mut ScreenRenderPostEvent<'_>) {
        crate::title_menu::render(host, event);
        guard("screen_render_post", || {
            if event.screen.id != "mod" || !ours(host) {
                return;
            }
            let cs = host.client_state();
            let gui = cs.as_ref().map_or([427.0, 240.0], |c| c.gui_size);
            let mut st = lock();
            match st.ui.mode {
                Some(Mode::Overlay) => {
                    let server = cs.as_ref().and_then(|c| c.server_address.clone());
                    let layout = overlay::layout(&View {
                        tab: st.ui.tab,
                        social: &st.social.snapshot,
                        signed_in: st.social.signed_in,
                        server: server.as_deref(),
                        screen: gui,
                    });
                    for d in &layout.draws {
                        match d {
                            Draw::Fill { rect, argb } => host.fill(event.batch, *rect, *argb),
                            Draw::Text { at, text, rgb } => {
                                host.text(event.batch, text, *at, *rgb, true)
                            }
                        }
                    }
                    st.ui.layout = layout;
                }
                Some(Mode::Wheel) => {
                    let items = wheel_items(&st.social.snapshot.emotes);
                    let centre = [gui[0] / 2.0, gui[1] / 2.0];
                    let hover = wheel::pick(centre, event.screen.mouse, items.len());
                    if items.is_empty() {
                        let text = "No emotes yet: find them in Rustic Launcher's Wardrobe";
                        let w = host.text_width(text);
                        host.fill(
                            event.batch,
                            [centre[0] - w / 2.0 - 3.0, centre[1] - 6.0, w + 6.0, 12.0],
                            BOX,
                        );
                        host.text(
                            event.batch,
                            text,
                            [centre[0] - w / 2.0, centre[1] - 4.0],
                            0xFF_FFFF,
                            true,
                        );
                    }
                    for (i, item) in items.iter().enumerate() {
                        let (quad, label) = wheel::slice_quad(centre, i, items.len());
                        let argb = if hover == Some(i) {
                            0xC0E0_7A3C
                        } else {
                            0xA012_1018
                        };
                        host.quad(event.batch, quad, argb);
                        let name = st
                            .store
                            .get(&item.hash)
                            .map_or(item.id.clone(), |l| l.cosmetic.name.clone());
                        let w = host.text_width(&name);
                        host.text(
                            event.batch,
                            &name,
                            [label[0] - w / 2.0, label[1] - 4.0],
                            0xFF_FFFF,
                            true,
                        );
                    }
                }
                None => {}
            }
        });
    }

    fn screen_mouse_button_pressed_pre(
        &self,
        host: &dyn Host,
        event: &mut ScreenMouseButtonPressedPreEvent<'_>,
    ) -> Flow {
        if crate::title_menu::mouse(host, event) == Flow::Cancel {
            return Flow::Cancel;
        }
        guard_or("screen_mouse_button_pressed_pre", Flow::Continue, || {
            if event.screen.id != "mod" || !ours(host) || event.button != 0 {
                return Flow::Continue;
            }
            let mut st = lock();
            match st.ui.mode {
                Some(Mode::Overlay) => {
                    let action = st.ui.layout.hit(event.screen.mouse).cloned();
                    match action {
                        Some(Action::Tab(t)) => st.ui.tab = t,
                        Some(Action::Close) => st.ui.pending_close = true,
                        Some(Action::InviteToMyServer(uuid)) => {
                            let address = host.client_state().and_then(|c| c.server_address);
                            if let (Some(to), Some(address), Some(n)) =
                                (parse_uuid(&uuid), address, net())
                            {
                                n.send(Job::Invite { to, address });
                            }
                        }
                        Some(Action::Accept(id)) => {
                            if let Some(n) = net() {
                                n.send(Job::Accept(id));
                            }
                        }
                        None => {}
                    }
                }
                Some(Mode::Wheel) => {
                    let gui = host.client_state().map_or([427.0, 240.0], |c| c.gui_size);
                    let items = wheel_items(&st.social.snapshot.emotes);
                    let centre = [gui[0] / 2.0, gui[1] / 2.0];
                    if let Some(i) = wheel::pick(centre, event.screen.mouse, items.len()) {
                        play(&mut st, items[i].clone());
                    }
                    st.ui.pending_close = true;
                }
                None => return Flow::Continue,
            }
            Flow::Cancel
        })
    }

    fn screen_key_pressed_pre(
        &self,
        host: &dyn Host,
        event: &mut ScreenKeyPressedPreEvent<'_>,
    ) -> Flow {
        if crate::title_menu::key(host, event) == Flow::Cancel {
            return Flow::Cancel;
        }
        guard_or("screen_key_pressed_pre", Flow::Continue, || {
            if event.screen.id != "mod" || !ours(host) {
                return Flow::Continue;
            }
            let mut st = lock();
            if st.ui.mode == Some(Mode::Wheel) {
                let items = wheel_items(&st.social.snapshot.emotes);
                if let Some(i) = wheel::number_key(event.key, items.len()) {
                    play(&mut st, items[i].clone());
                    st.ui.pending_close = true;
                    return Flow::Cancel;
                }
            }
            Flow::Continue
        })
    }

    fn screen_closing(&self, host: &dyn Host, event: &mut ScreenClosingEvent<'_>) {
        guard("screen_closing", || {
            if event.screen.id == "mod" && host.mod_screen().is_none_or(|m| m == MOD_ID) {
                lock().ui.mode = None;
            }
        });
    }

    fn client_chat_received(
        &self,
        _host: &mut dyn Host,
        event: &mut ClientChatReceivedEvent<'_>,
    ) -> Flow {
        guard("client_chat_received", || {
            crate::badge::chat_line(&mut lock(), event.system, event.sender, event.message);
        });
        Flow::Continue
    }

    fn hud(&self, host: &mut dyn Host, event: &mut HudEvent<'_>) {
        guard("hud", || {
            let mut st = lock();
            // The badge is set at the size of the text the HUD draws.
            st.badges.scale = Some(event.batch.scale);
            let invites = st.social.snapshot.invites.len();
            if invites == 0 || st.ui.mode.is_some() {
                return;
            }
            let text = format!(
                "Rustic: {invites} invite{} ({})",
                if invites == 1 { "" } else { "s" },
                st.cfg
                    .overlay_key
                    .rsplit('.')
                    .next()
                    .unwrap_or("")
                    .to_uppercase()
            );
            drop(st);
            let w = host.text_width(&text) + 6.0;
            let (x, y) = event.corners.place(Corner::TopRight, w, 12.0);
            host.fill(event.batch, [x, y, w, 12.0], BOX);
            host.text(event.batch, &text, [x + 3.0, y + 2.0], 0xFF_FFFF, true);
        });
    }

    fn client_disconnecting(&self, _host: &dyn Host, _event: &mut ClientDisconnectingEvent) {
        guard("client_disconnecting", || {
            let mut st = lock();
            st.trails.clear();
            st.in_view.clear();
            st.others.clear();
            st.own_emote = None;
        });
    }
}

/// The wheel's emotes: the owned ones, at most 8.
fn wheel_items(owned: &[WornItem]) -> Vec<WornItem> {
    owned.iter().take(wheel::MAX_SLICES).cloned().collect()
}

/// Starts an emote on the window's own player and tells the cloud (others see it).
fn play(st: &mut crate::state::State, item: WornItem) {
    if let Some(n) = net() {
        n.send(Job::Playing(Some(item.id.clone())));
    }
    st.own_emote = Some(OwnEmote { item, pose: None });
}

fn client_tick(host: &mut dyn Host) {
    let Some(cs) = host.client_state() else {
        return;
    };
    let now = Instant::now();
    if !lock().started {
        let text = host.config_dir().ok().and_then(|d| {
            std::fs::read_to_string(d.join(rusticmc_companion_core::config::FILE)).ok()
        });
        lock().start(text);
    }
    let Some(net) = net() else {
        return;
    };
    // The players the list shows, for the badge (read without the lock: a host call).
    let listed = lock().badges.list_due(now).then(|| {
        host.client_profiles()
            .iter()
            .map(|p| p.uuid)
            .collect::<Vec<_>>()
    });
    let (close, toast) = {
        let mut st = lock();
        if let Some(listed) = listed {
            st.badges.listed = listed;
            st.badges.listed_at = Some(now);
        }
        for d in net.drain() {
            st.apply(d, now);
        }
        let close = std::mem::take(&mut st.ui.pending_close);
        if close {
            st.ui.mode = None;
        }
        // What the players in view wear, then the bundles it names (and the emotes they and
        // the player play): all asked off this thread.
        let in_view = st.in_view.clone();
        // Who uses Rustic among the listed players, those in view and chat senders.
        let batch = st.badges.next_batch(&in_view, now);
        if !batch.is_empty() {
            net.send(Job::Badges(batch));
        }
        if !in_view.is_empty() {
            let batch = st.worn.next_batch(&in_view, now);
            if !batch.is_empty() {
                net.send(Job::Worn(batch));
            }
        }
        let mut wanted: Vec<WornItem> = Vec::new();
        for u in &in_view {
            wanted.extend(st.worn.worn(*u).iter().cloned());
            if let Some((item, _)) = st.worn.playing(*u, now) {
                wanted.push(item.clone());
            }
        }
        wanted.extend(wheel_items(&st.social.snapshot.emotes));
        for item in wanted {
            if st.store.want(&item.hash, now) {
                net.send(Job::Bundle(item));
            }
        }
        let overlay_open = st.ui.mode == Some(Mode::Overlay);
        if st.social.due(overlay_open, now) {
            net.send(Job::Social);
        }
        // One discreet toast at a time, only while playing.
        let toast = (cs.screen == NativeScreenKind::Playing && st.cfg.notifications)
            .then(|| st.notes.next(now))
            .flatten();
        (close, toast)
    };
    // The lock is released: these may run other hooks (`toast_add`, screen hooks).
    if close {
        host.mod_screen_close(MOD_ID);
    }
    if let Some(n) = toast {
        host.add_toast(
            &n.id,
            "minecraft:name_tag",
            Text::literal(n.title),
            Text::literal(n.body),
            false,
        );
    }
}
