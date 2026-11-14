//! Rustic's Create World form (ADR 0165): the game's Create New World screen (its Game, World
//! and More tabs) as one launcher form over it — the world's name, the game mode and the
//! difficulty as segments, cheats as a switch; then the seed, the world type with its Customize
//! button, structures and the bonus chest; then Game Rules, Experiments and Data Packs, which
//! open the game's own screens. Cancel and Create New World in the footer. Pure: the game crate
//! gives the choices and does the presses through the screen itself (Rustic's Horizon patch U19).
use crate::kit::{Kit, Look, max_scroll};
use crate::paint::{Align, Measure, Paint, inside};
use crate::sheet::{self, FOOTER, HEADER};
use crate::skin::palette;
use crate::title::Weight;

/// What the form shows (the game's choices, its words already translated).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Form {
    pub name: String,
    pub seed: String,
    /// 0 Survival, 1 Hardcore, 2 Creative.
    pub game_mode: u8,
    /// 0 Peaceful .. 3 Hard.
    pub difficulty: u8,
    pub commands: bool,
    pub world_type: String,
    pub customizable: bool,
    pub structures: bool,
    pub bonus_chest: bool,
    /// Hardcore: difficulty, cheats and the bonus chest are fixed.
    pub hardcore: bool,
    pub data_packs: usize,
}

/// The form's words.
#[derive(Clone, Debug, Default)]
pub struct Labels {
    pub title: String,
    pub game: String,
    pub world: String,
    pub more: String,
    pub name: String,
    pub game_mode: String,
    /// Survival, Hardcore, Creative.
    pub modes: [String; 3],
    pub difficulty: String,
    /// Peaceful, Easy, Normal, Hard.
    pub difficulties: [String; 4],
    pub commands: String,
    pub seed: String,
    pub seed_hint: String,
    pub world_type: String,
    pub customize: String,
    pub structures: String,
    pub bonus_chest: String,
    pub game_rules: String,
    pub experiments: String,
    pub data_packs: String,
    pub create: String,
    pub cancel: String,
}

/// A text field of the form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Name,
    Seed,
}

/// What a press lands on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Hit {
    Field(Field),
    Mode(u8),
    Difficulty(u8),
    Commands,
    TypePrev,
    TypeNext,
    Customize,
    Structures,
    BonusChest,
    GameRules,
    Experiments,
    DataPacks,
    Create,
    Cancel,
    Nothing,
}

/// What the form shows this frame.
pub struct View<'a> {
    pub gui: [f32; 2],
    pub mouse: [f32; 2],
    pub measure: Measure<'a>,
    pub labels: &'a Labels,
    pub form: &'a Form,
    /// The field taking typing.
    pub typing: Option<Field>,
    pub caret: bool,
    pub scroll: f32,
    /// The keyboard's stop (an index in [`Layout::stops`]).
    pub focus: Option<usize>,
}

/// What was laid out.
#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub paints: Vec<Paint>,
    pub hits: Vec<([f32; 4], Hit)>,
    pub area: [f32; 4],
    pub max_scroll: f32,
    /// Tab's stops, in order: a row's box and what Enter presses on it.
    pub stops: Vec<([f32; 4], Hit)>,
}

impl Layout {
    pub fn hit(&self, at: [f32; 2]) -> Option<Hit> {
        self.hits
            .iter()
            .rev()
            .find(|(r, _)| inside(*r, at))
            .map(|(_, h)| h.clone())
    }
}

const ROW: f32 = 24.0;

struct Rows<'k, 'a> {
    k: Kit<'a, Hit>,
    x: f32,
    w: f32,
    y: f32,
    area: [f32; 4],
    stops: Vec<([f32; 4], Hit)>,
    focus: Option<usize>,
    _p: std::marker::PhantomData<&'k ()>,
}

impl Rows<'_, '_> {
    fn visible(&self, r: [f32; 4]) -> bool {
        r[1] + r[3] > self.area[1] && r[1] < self.area[1] + self.area[3]
    }
    fn focused(&self) -> bool {
        self.focus == Some(self.stops.len())
    }
    fn section(&mut self, name: &str) {
        self.y += 6.0;
        if self.visible([self.x, self.y, self.w, 12.0]) {
            self.k.text(
                self.x + 2.0,
                self.y + 6.0,
                name,
                7.0,
                Weight::SemiBold,
                palette::ACCENT_TEXT,
                Align::Left,
            );
        }
        self.y += 14.0;
    }
    /// A row's plate and name; the control's box (its right part).
    fn row(&mut self, name: &str, on: bool) -> ([f32; 4], [f32; 4]) {
        let r = [self.x, self.y, self.w, ROW - 3.0];
        self.y += ROW;
        let lit = self.k.over(r) && inside(self.area, self.k.mouse);
        let focused = self.focused();
        self.k.round(
            r,
            5.0,
            if lit || focused {
                palette::FACE_HOVER
            } else {
                palette::FACE
            },
            if focused {
                palette::ACCENT
            } else {
                palette::BORDER
            },
        );
        let name_w = (self.w * 0.38).max(70.0);
        self.k.text_fit(
            r[0] + 9.0,
            r[1] + r[3] / 2.0,
            name_w - 12.0,
            name,
            7.0,
            Weight::Medium,
            if on { palette::INK } else { palette::INK_OFF },
        );
        let c = [r[0] + name_w, r[1] + 3.0, r[2] - name_w - 6.0, r[3] - 6.0];
        (r, c)
    }
    fn stop(&mut self, r: [f32; 4], h: Hit) {
        self.stops.push((r, h));
    }
}

fn segments(
    rows: &mut Rows<'_, '_>,
    c: [f32; 4],
    labels: &[String],
    at: u8,
    on: bool,
    hit: impl Fn(u8) -> Hit,
) {
    let n = labels.len().max(1) as f32;
    let w = (c[2] - 2.0 * (n - 1.0)) / n;
    for (i, l) in labels.iter().enumerate() {
        let r = [c[0] + i as f32 * (w + 2.0), c[1], w, c[3]];
        let chosen = i as u8 == at;
        let lit = on && rows.k.over(r);
        let (fill, border, ink) = match (on, chosen, lit) {
            (false, true, _) => (palette::FACE_OFF, palette::BORDER_OFF, palette::INK_OFF),
            (false, false, _) => (0, 0, palette::INK_OFF),
            (true, true, _) => (palette::ACCENT_SOFT, palette::ACCENT, palette::ACCENT_TEXT),
            (true, false, true) => (palette::FACE_HOVER, 0x33FF_FFFF, palette::INK),
            (true, false, false) => (0x80_0D0E11, palette::BORDER, palette::MUTED),
        };
        rows.k.round(r, 4.0, fill, border);
        let label = crate::paint::fit(l, r[2] - 4.0, 6.5, Weight::SemiBold, rows.k.measure);
        rows.k.text(
            r[0] + r[2] / 2.0,
            r[1] + r[3] / 2.0,
            &label,
            6.5,
            Weight::SemiBold,
            ink,
            Align::Center,
        );
        if on {
            rows.k.hit(r, hit(i as u8));
        }
    }
}

/// Lays the form out.
pub fn layout(v: &View<'_>) -> Layout {
    let [gw, gh] = v.gui;
    let l = v.labels;
    let f = v.form;
    let mut k: Kit<'_, Hit> = Kit::new(v.measure, v.mouse);
    k.hit([0.0, 0.0, gw, gh], Hit::Nothing);
    k.fill([0.0, 0.0, gw, gh], 0xFF0B_0C0F);
    sheet::title(&mut k, gw, &l.title);
    let w = (gw - 24.0).min(440.0);
    let x = ((gw - w) / 2.0).round();
    let area = [
        x - 4.0,
        HEADER - 22.0,
        w + 8.0,
        gh - FOOTER - 6.0 - (HEADER - 22.0),
    ];
    let mut rows = Rows {
        k,
        x,
        w,
        y: area[1] - v.scroll,
        area,
        stops: Vec::new(),
        focus: v.focus,
        _p: std::marker::PhantomData,
    };
    rows.k.out.push(Paint::ClipBegin(area));
    let hits_from = rows.k.hits.len();
    // Game.
    rows.section(&l.game);
    let (r, c) = rows.row(&l.name, true);
    let typing = v.typing == Some(Field::Name);
    sheet::input(
        &mut rows.k,
        c,
        &f.name,
        &l.name,
        typing,
        v.caret,
        Hit::Field(Field::Name),
    );
    rows.stop(r, Hit::Field(Field::Name));
    let (r, c) = rows.row(&l.game_mode, true);
    segments(&mut rows, c, &l.modes, f.game_mode, true, Hit::Mode);
    rows.stop(r, Hit::Mode(f.game_mode));
    let (r, c) = rows.row(&l.difficulty, !f.hardcore);
    segments(
        &mut rows,
        c,
        &l.difficulties,
        f.difficulty,
        !f.hardcore,
        Hit::Difficulty,
    );
    if !f.hardcore {
        rows.stop(r, Hit::Difficulty(f.difficulty));
    }
    let (r, c) = rows.row(&l.commands, !f.hardcore);
    sheet::switch(
        &mut rows.k,
        c[0] + c[2],
        c[1] + c[3] / 2.0,
        f.commands,
        !f.hardcore,
        Hit::Commands,
    );
    if !f.hardcore {
        rows.stop(r, Hit::Commands);
    }
    // World.
    rows.section(&l.world);
    let (r, c) = rows.row(&l.seed, true);
    let typing = v.typing == Some(Field::Seed);
    sheet::input(
        &mut rows.k,
        c,
        &f.seed,
        &l.seed_hint,
        typing,
        v.caret,
        Hit::Field(Field::Seed),
    );
    rows.stop(r, Hit::Field(Field::Seed));
    let (r, c) = rows.row(&l.world_type, true);
    let cw = if f.customizable {
        rows.k.button_width(&l.customize, c[3]).min(c[2] * 0.45)
    } else {
        0.0
    };
    let stepper = [
        c[0],
        c[1],
        c[2] - if cw > 0.0 { cw + 4.0 } else { 0.0 },
        c[3],
    ];
    let bs = stepper[3];
    // The stepper's arrows: the type before and after.
    let prev = [stepper[0], stepper[1], bs, bs];
    let next = [stepper[0] + stepper[2] - bs, stepper[1], bs, bs];
    for (rr, glyph, hit) in [(prev, "‹", Hit::TypePrev), (next, "›", Hit::TypeNext)] {
        rows.k.button(rr, glyph, Look::Secondary, false, hit);
    }
    rows.k.text(
        stepper[0] + stepper[2] / 2.0,
        stepper[1] + stepper[3] / 2.0,
        &crate::paint::fit(
            &f.world_type,
            stepper[2] - 2.0 * bs - 6.0,
            7.0,
            Weight::SemiBold,
            rows.k.measure,
        ),
        7.0,
        Weight::SemiBold,
        palette::INK,
        Align::Center,
    );
    if f.customizable {
        rows.k.button(
            [c[0] + c[2] - cw, c[1], cw, c[3]],
            &l.customize,
            Look::Secondary,
            false,
            Hit::Customize,
        );
    }
    rows.stop(r, Hit::TypeNext);
    let (r, c) = rows.row(&l.structures, true);
    sheet::switch(
        &mut rows.k,
        c[0] + c[2],
        c[1] + c[3] / 2.0,
        f.structures,
        true,
        Hit::Structures,
    );
    rows.stop(r, Hit::Structures);
    let (r, c) = rows.row(&l.bonus_chest, !f.hardcore);
    sheet::switch(
        &mut rows.k,
        c[0] + c[2],
        c[1] + c[3] / 2.0,
        f.bonus_chest,
        !f.hardcore,
        Hit::BonusChest,
    );
    if !f.hardcore {
        rows.stop(r, Hit::BonusChest);
    }
    // More: the game's own screens.
    rows.section(&l.more);
    for (name, hit, value) in [
        (&l.game_rules, Hit::GameRules, String::new()),
        (&l.experiments, Hit::Experiments, String::new()),
        (
            &l.data_packs,
            Hit::DataPacks,
            if f.data_packs > 0 {
                f.data_packs.to_string()
            } else {
                String::new()
            },
        ),
    ] {
        let (r, c) = rows.row(name, true);
        rows.k.text(
            c[0] + c[2],
            c[1] + c[3] / 2.0,
            &format!("{value}  ›"),
            8.0,
            Weight::SemiBold,
            palette::MUTED,
            Align::Right,
        );
        rows.k.hit(r, hit.clone());
        rows.stop(r, hit);
    }
    let content = rows.y + v.scroll - area[1] + 6.0;
    rows.k.out.push(Paint::ClipEnd);
    // Hit boxes outside the form's view are cut to it.
    for (hr, _) in &mut rows.k.hits[hits_from..] {
        let y0 = hr[1].max(area[1]);
        let y1 = (hr[1] + hr[3]).min(area[1] + area[3]);
        hr[1] = y0;
        hr[3] = (y1 - y0).max(0.0);
    }
    let max = max_scroll(content, area[3]);
    rows.k.scrollbar(area, content, v.scroll.clamp(0.0, max));
    // The footer.
    let mut k = rows.k;
    let mut stops = rows.stops;
    let cy = sheet::footer(&mut k, gw, gh);
    let h = sheet::BUTTON_H;
    let crw = k.button_width(&l.create, h).max(100.0);
    let cnw = k.button_width(&l.cancel, h).max(60.0);
    let n = stops.len();
    let cancel = [x, cy - h / 2.0, cnw, h];
    let create = [x + w - crw, cy - h / 2.0, crw, h];
    k.button(
        cancel,
        &l.cancel,
        Look::Ghost,
        v.focus == Some(n),
        Hit::Cancel,
    );
    k.button(
        create,
        &l.create,
        Look::Primary,
        v.focus == Some(n + 1),
        Hit::Create,
    );
    stops.push((cancel, Hit::Cancel));
    stops.push((create, Hit::Create));
    Layout {
        paints: k.out,
        hits: k.hits,
        area,
        max_scroll: max,
        stops,
    }
}

/// The step count to go from `from` to `to` forward in a cycle of `n` (the game's buttons cycle).
pub fn steps(from: u8, to: u8, n: u8) -> u8 {
    if n == 0 {
        return 0;
    }
    (to + n - from % n) % n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint::test_measure;

    #[test]
    fn the_form_lays_out_every_choice() {
        let labels = Labels {
            create: "Create New World".into(),
            ..Labels::default()
        };
        let form = Form {
            name: "New World".into(),
            customizable: true,
            ..Form::default()
        };
        let v = View {
            gui: [854.0, 900.0],
            mouse: [0.0, 0.0],
            measure: &test_measure,
            labels: &labels,
            form: &form,
            typing: Some(Field::Name),
            caret: true,
            scroll: 0.0,
            focus: None,
        };
        let l = layout(&v);
        for h in [
            Hit::Mode(2),
            Hit::Difficulty(3),
            Hit::Commands,
            Hit::TypeNext,
            Hit::Customize,
            Hit::BonusChest,
            Hit::DataPacks,
            Hit::Create,
        ] {
            assert!(l.hits.iter().any(|(_, x)| *x == h), "{h:?}");
        }
        assert_eq!(l.stops.last().map(|s| &s.1), Some(&Hit::Create));
        // Hardcore: difficulty, cheats and the bonus chest take no press.
        let hard = Form {
            hardcore: true,
            game_mode: 1,
            ..form.clone()
        };
        let l = layout(&View { form: &hard, ..v });
        assert!(!l.hits.iter().any(|(_, x)| matches!(x, Hit::Difficulty(_))));
        assert!(!l.hits.iter().any(|(_, x)| *x == Hit::BonusChest));
        assert_eq!((steps(0, 2, 3), steps(2, 0, 3), steps(1, 1, 4)), (2, 1, 0));
    }
}
