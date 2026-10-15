//! Стан учня: прогрес навичок, розклад повторень, показані умови, лічильники помилок.
//! Лише цілі й BTree-колекції: порядок обходу однаковий на всіх платформах. Ніяких імен чи часу годинника.

use crate::bkt::{BktParams, SkillState};
use crate::config::Config;
use crate::review::ReviewSchedule;
mod codec;

pub use codec::CodecError;

use std::collections::{BTreeMap, BTreeSet};

/// Навичка, яку пора повторити.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DueReview {
    pub skill: String,
    pub overdue_days: u32,
    /// `true`, якщо пропуск довгий і замість повторення пропонується короткий мікротест.
    pub microtest: bool,
}

/// Хід діагностичного мікротесту (після довгого пропуску замість звичайного повторення).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MicrotestOutcome {
    InProgress,
    /// Достатньо чистих результатів: навичка збережена, інтервали повторень починаються спочатку.
    Passed,
    /// Замало чистих результатів: навичка повертається до навчання.
    Failed,
}

/// Що сталося після запису результату задачі.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskRecord {
    pub newly_mastered: bool,
    /// Це було планове повторення вже опанованої навички.
    pub was_review: bool,
    /// Опанування знято: оцінка впала нижче порога відкату або мікротест не пройдено. Стікери лишаються.
    pub rolled_back: bool,
    /// Якщо це задача мікротесту, то його стан.
    pub microtest: Option<MicrotestOutcome>,
}

#[derive(Clone, Debug, Default)]
pub struct LearnerState {
    skills: BTreeMap<String, SkillState>,
    mastered: BTreeSet<String>,
    schedules: BTreeMap<String, ReviewSchedule>,
    seen: BTreeMap<String, BTreeSet<String>>,
    seen_order: BTreeMap<String, Vec<String>>,
    cursors: BTreeMap<String, u64>,
    exhausted: BTreeSet<String>,
    bug_counts: BTreeMap<(String, String), u32>,
    tasks_today: BTreeMap<String, u32>,
    reviews_today: usize,
    ask_adult: Vec<(String, String)>,
    microtests: BTreeMap<String, (u8, u8)>,
    today: u32,
}

impl LearnerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn today(&self) -> u32 {
        self.today
    }

    /// Початок дня: лічильники «за сьогодні» обнуляються лише при зміні дня.
    pub fn begin_day(&mut self, day: u32) {
        if day != self.today {
            self.tasks_today.clear();
            self.reviews_today = 0;
            self.today = day;
        }
    }

    pub fn skill(&self, id: &str) -> Option<&SkillState> {
        self.skills.get(id)
    }

    /// Опанування «липке»: після досягнення навичка лишається опанованою, а забування обробляють повторення.
    pub fn is_mastered(&self, id: &str) -> bool {
        self.mastered.contains(id)
    }

    /// Лише для тестів: позначає тему опанованою.
    #[cfg(feature = "test-support")]
    pub fn force_mastered_for_test(&mut self, id: &str) {
        self.mastered.insert(id.to_string());
    }

    pub fn mastered_skills(&self) -> Vec<String> {
        self.mastered.iter().cloned().collect()
    }

    pub fn schedule(&self, id: &str) -> Option<&ReviewSchedule> {
        self.schedules.get(id)
    }

    /// Скільки задач усіх навичок сьогодні (номер наступної задачі дня для слота перемежування).
    pub fn tasks_today_total(&self) -> u32 {
        self.tasks_today
            .values()
            .fold(0u32, |a, n| a.saturating_add(*n))
    }

    pub fn tasks_today(&self, id: &str) -> u32 {
        self.tasks_today.get(id).copied().unwrap_or(0)
    }

    pub fn bug_count(&self, skill: &str, bug: &str) -> u32 {
        self.bug_counts
            .get(&(skill.to_string(), bug.to_string()))
            .copied()
            .unwrap_or(0)
    }

    /// Гіпотези про помилки, що повторилися щонайменше `min` разів (одиничний збіг гіпотезою не вважається).
    pub fn bug_hypotheses(&self, skill: &str, min: u32) -> Vec<String> {
        self.bug_counts
            .iter()
            .filter(|((s, _), n)| s == skill && **n >= min)
            .map(|((_, b), _)| b.clone())
            .collect()
    }

    /// Записує результат задачі. `bug` — id типової помилки, якщо хоч одна відповідь на цій задачі їй відповідала
    /// (незалежно від того, чи розв'язано задачу зрештою).
    pub fn record_task(
        &mut self,
        params: &BktParams,
        cfg: &Config,
        skill: &str,
        correct: bool,
        hints_used: u8,
        bug: Option<&str>,
    ) -> TaskRecord {
        let today = self.today;
        let count = self.tasks_today.entry(skill.to_string()).or_insert(0);
        *count = count.saturating_add(1);

        let was_mastered = self.mastered.contains(skill);
        let state = self
            .skills
            .entry(skill.to_string())
            .or_insert_with(|| SkillState::new(params));
        state.observe(params, today, correct, hints_used);
        let now_mastered = state.is_mastered(cfg);

        // Помилка зараховується, якщо на цій задачі її допустили хоч раз, навіть коли дитина зрештою розв'язала
        // (з драбиною це звичайний шлях): саме з цих збігів учителю будуються гіпотези.
        if let Some(b) = bug {
            let n = self
                .bug_counts
                .entry((skill.to_string(), b.to_string()))
                .or_insert(0);
            *n = n.saturating_add(1);
        }

        let clean = correct && hints_used == 0;
        let p_after = self.skills.get(skill).map_or(0.0, |s| s.p_known);
        let mut record = TaskRecord {
            newly_mastered: false,
            was_review: false,
            rolled_back: false,
            microtest: None,
        };
        if was_mastered {
            if p_after < cfg.rollback_p {
                // Відкат за специфікацією: навичка знову потребує навчання, розклад повторень знято.
                self.demote(skill);
                record.rolled_back = true;
            } else if let Some(sch) = self.schedules.get(skill).copied() {
                if sch.is_due(today) {
                    record.was_review = true;
                    self.reviews_today = self.reviews_today.saturating_add(1);
                    if sch.needs_microtest(today, cfg) {
                        record.microtest = Some(self.step_microtest(skill, clean, cfg));
                        match record.microtest {
                            Some(MicrotestOutcome::Passed) => {
                                self.schedules.insert(
                                    skill.to_string(),
                                    ReviewSchedule::after_mastery(today, cfg),
                                );
                            }
                            Some(MicrotestOutcome::Failed) => {
                                self.demote(skill);
                                record.rolled_back = true;
                            }
                            _ => {}
                        }
                    } else {
                        self.schedules
                            .insert(skill.to_string(), sch.after_review(clean, today, cfg));
                    }
                }
            }
        } else if now_mastered {
            self.mastered.insert(skill.to_string());
            self.schedules
                .insert(skill.to_string(), ReviewSchedule::after_mastery(today, cfg));
            record.newly_mastered = true;
            // Опанована навичка більше не характеризується давніми помилками: докази починаються спочатку.
            self.bug_counts.retain(|(s, _), _| s != skill);
        }
        record
    }

    /// Опанування за результатом перевірки «я це вже вмію» (три чисті задачі без підказок). Повертає `true`, якщо
    /// навичка стала опанованою саме зараз. Повторення розкладається як після звичайного опанування.
    pub fn grant_mastery(&mut self, params: &BktParams, cfg: &Config, skill: &str) -> bool {
        if self.mastered.contains(skill) {
            return false;
        }
        let state = self
            .skills
            .entry(skill.to_string())
            .or_insert_with(|| SkillState::new(params));
        state.p_known = state.p_known.max(cfg.mastery_p + 0.02).min(0.99);
        state.streak = state.streak.max(cfg.mastery_streak);
        state.attempts = state.attempts.max(cfg.mastery_streak);
        self.mastered.insert(skill.to_string());
        self.schedules.insert(
            skill.to_string(),
            ReviewSchedule::after_mastery(self.today, cfg),
        );
        self.bug_counts.retain(|(s, _), _| s != skill);
        true
    }

    /// Знімає опанування (стікери в альбомі не чіпає: вони належать альбому).
    fn demote(&mut self, skill: &str) {
        self.mastered.remove(skill);
        self.schedules.remove(skill);
        self.microtests.remove(skill);
    }

    /// Враховує одну задачу мікротесту й повідомляє, чим він закінчився.
    fn step_microtest(&mut self, skill: &str, clean: bool, cfg: &Config) -> MicrotestOutcome {
        let entry = self.microtests.entry(skill.to_string()).or_insert((0, 0));
        entry.0 = entry.0.saturating_add(1);
        if clean {
            entry.1 = entry.1.saturating_add(1);
        }
        let (done, clean_count) = *entry;
        if done < cfg.microtest_tasks {
            return MicrotestOutcome::InProgress;
        }
        self.microtests.remove(skill);
        if clean_count >= cfg.microtest_pass_clean {
            MicrotestOutcome::Passed
        } else {
            MicrotestOutcome::Failed
        }
    }

    /// Прострочені повторення: найстарші першими. За день не більше `max_reviews_per_session` повторень
    /// разом (без «лавини боргу» після пропусків).
    pub fn due_reviews(&self, cfg: &Config) -> Vec<DueReview> {
        let mut due: Vec<DueReview> = self
            .schedules
            .iter()
            .filter(|(skill, sch)| sch.is_due(self.today) && !self.exhausted.contains(*skill))
            .map(|(skill, sch)| DueReview {
                skill: skill.clone(),
                overdue_days: sch.overdue_days(self.today),
                microtest: sch.needs_microtest(self.today, cfg),
            })
            .collect();
        due.sort_by(|a, b| {
            b.overdue_days
                .cmp(&a.overdue_days)
                .then_with(|| a.skill.cmp(&b.skill))
        });
        due.truncate(
            cfg.max_reviews_per_session
                .saturating_sub(self.reviews_today),
        );
        due
    }

    /// Усі прострочені повторення без денного ліміту (для режиму «Повторення»): найстарші першими.
    pub fn all_due_reviews(&self, cfg: &Config) -> Vec<DueReview> {
        let mut due: Vec<DueReview> = self
            .schedules
            .iter()
            .filter(|(skill, sch)| sch.is_due(self.today) && !self.exhausted.contains(*skill))
            .map(|(skill, sch)| DueReview {
                skill: skill.clone(),
                overdue_days: sch.overdue_days(self.today),
                microtest: sch.needs_microtest(self.today, cfg),
            })
            .collect();
        due.sort_by(|a, b| {
            b.overdue_days
                .cmp(&a.overdue_days)
                .then_with(|| a.skill.cmp(&b.skill))
        });
        due
    }

    /// Міцність пам'яті про опановану тему, відсотки 0..=100 (100 одразу після повторення, близько 34 у день, коли пора
    /// повторити, 0 через півтора інтервалу). `None`, якщо теми немає в розкладі.
    pub fn memory_pct(&self, skill: &str, cfg: &Config) -> Option<u32> {
        let sch = self.schedules.get(skill)?;
        let interval = cfg.review_intervals_days[usize::from(sch.level())].max(1);
        let elapsed = (self.today.saturating_add(interval)).saturating_sub(sch.due_day());
        let lost = u64::from(elapsed) * 200 / (u64::from(interval) * 3);
        Some(u32::try_from(100u64.saturating_sub(lost)).unwrap_or(0))
    }

    /// Розклад повторень: (тема, рівень, день наступного повторення).
    pub fn schedule_list(&self) -> Vec<(String, u8, u32)> {
        self.schedules
            .iter()
            .map(|(s, sch)| (s.clone(), sch.level(), sch.due_day()))
            .collect()
    }

    /// Запам'ятовує показану умову. `true`, якщо вона нова для цієї навички.
    pub fn mark_seen(&mut self, skill: &str, statement: &str) -> bool {
        let inserted = self
            .seen
            .entry(skill.to_string())
            .or_default()
            .insert(statement.to_string());
        if inserted {
            self.seen_order
                .entry(skill.to_string())
                .or_default()
                .push(statement.to_string());
        }
        inserted
    }

    /// Нове коло умов: забуває всі показані умови навички, крім `keep_recent` останніх, щоб вони
    /// не повторювалися одразу. `false`, якщо забувати нічого (показано не більше, ніж треба тримати).
    pub fn recycle_seen(&mut self, skill: &str, keep_recent: usize) -> bool {
        let Some(order) = self.seen_order.get_mut(skill) else {
            return false;
        };
        if order.len() <= keep_recent {
            return false;
        }
        let forget = order.len() - keep_recent;
        let forgotten: Vec<String> = order.drain(..forget).collect();
        if let Some(set) = self.seen.get_mut(skill) {
            for s in &forgotten {
                set.remove(s);
            }
        }
        true
    }

    pub fn has_seen(&self, skill: &str, statement: &str) -> bool {
        self.seen.get(skill).is_some_and(|s| s.contains(statement))
    }

    /// Наступний номер варіанта для навички (1, 2, 3, ...).
    pub fn next_variant(&mut self, skill: &str) -> u64 {
        let c = self.cursors.entry(skill.to_string()).or_insert(0);
        *c = c.saturating_add(1);
        *c
    }

    /// Задача, з якою потрібна людина (драбину пройдено): навичка й умова, без розв'язку.
    pub fn record_ask_adult(&mut self, skill: &str, statement: &str) {
        let item = (skill.to_string(), statement.to_string());
        if !self.ask_adult.contains(&item) {
            self.ask_adult.push(item);
        }
    }

    /// Список «запитай дорослого»: пари (навичка, умова) у порядку додавання (останні внизу).
    pub fn ask_adult_items(&self) -> Vec<(String, String)> {
        self.ask_adult.clone()
    }

    /// Дорослий допоміг із цією задачею (відкрите питання Q5: хто і як це позначає).
    pub fn resolve_ask_adult(&mut self, skill: &str, statement: &str) -> bool {
        let item = (skill.to_string(), statement.to_string());
        match self.ask_adult.iter().position(|x| *x == item) {
            Some(i) => {
                self.ask_adult.remove(i);
                true
            }
            None => false,
        }
    }

    /// Простір умов навички вичерпано: селектор її більше не пропонує.
    pub fn mark_exhausted(&mut self, skill: &str) {
        self.exhausted.insert(skill.to_string());
    }

    pub fn is_exhausted(&self, skill: &str) -> bool {
        self.exhausted.contains(skill)
    }
}
