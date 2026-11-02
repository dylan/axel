"use client";

import { useEffect, useState } from "react";
import {
  ArrowRight,
  BadgeCheck,
  Coins,
  GitBranch,
  KeyRound,
  Laptop,
  MessagesSquare,
  Server,
  ShieldCheck,
  Terminal,
} from "lucide-react";
import { DOCS, REPO } from "@/lib/site";

const PROMPT = "add retries to the webhook sender @aaron";

/* ---------------------------------------------------------------- terminal */

type Agent = { name: string; color: string };
const AARON: Agent = { name: "aaron", color: "text-tangerine" };
const JUDE: Agent = { name: "jude", color: "text-sky" };
const KIT: Agent = { name: "kit", color: "text-sun" };

/** The prompt text, with `@name`s of known agents in their colours. */
function Mentions({ text }: { text: string }) {
  const parts = text.split(/(@[a-z]+)/g);
  return (
    <>
      {parts.map((p, i) => {
        const agent = [AARON, JUDE, KIT].find((a) => p === `@${a.name}`);
        return agent ? (
          <span key={i} className={`${agent.color} font-semibold`}>
            {p}
          </span>
        ) : (
          <span key={i}>{p}</span>
        );
      })}
    </>
  );
}

function useTyped(text: string, startMs = 600, stepMs = 45) {
  const [n, setN] = useState(0);
  useEffect(() => {
    if (window.matchMedia("(prefers-reduced-motion: reduce)").matches) {
      setN(text.length);
      return;
    }
    let i = 0;
    let timer: ReturnType<typeof setTimeout>;
    const tick = () => {
      i += 1;
      setN(i);
      if (i < text.length) timer = setTimeout(tick, stepMs);
    };
    timer = setTimeout(tick, startMs);
    return () => clearTimeout(timer);
  }, [text, startMs, stepMs]);
  return text.slice(0, n);
}

const THREADS: {
  glyph: string;
  glyphClass: string;
  title: string;
  agents: Agent[];
  project: string;
  tokens: string;
  age: string;
  settled?: boolean;
}[] = [
  { glyph: "●", glyphClass: "text-amber-400", title: "Migrate billing to Stripe", agents: [JUDE, KIT], project: "billing", tokens: "120k", age: "12m ago" },
  { glyph: "○", glyphClass: "text-white/40", title: "Rate-limit the public API", agents: [AARON], project: "api", tokens: "41k", age: "2h ago" },
  { glyph: "◐", glyphClass: "text-rose-400", title: "Flaky auth test on CI", agents: [AARON, JUDE], project: "api", tokens: "18k", age: "yesterday" },
  { glyph: "✓", glyphClass: "text-white/30", title: "Dark mode for the dashboard", agents: [KIT], project: "web", tokens: "64k", age: "3d ago", settled: true },
];

/** The two floating panes `axel` opens in Rex: the prompt, recent threads below. */
function Launcher() {
  const typed = useTyped(PROMPT);
  const done = typed.length === PROMPT.length;
  return (
    <div className="w-full overflow-hidden rounded-[11px] bg-surface text-left font-mono text-[13px] leading-6 shadow-2xl shadow-black/40 backdrop-blur">
      <div className="flex items-center gap-2 border-b border-line px-4 py-2 text-white/50">
        <Terminal className="h-3.5 w-3.5" />
        <span>why not today?</span>
      </div>
      <div className="flex items-center justify-between gap-4 px-5 py-4">
        <div className="min-w-0 truncate text-white">
          <span className="text-pink">› </span>
          {typed ? <Mentions text={typed} /> : <span className="text-white/35">What are we building?</span>}
          <span className="ml-px inline-block h-[1.1em] w-[2px] translate-y-[3px] bg-pink animate-[blink_1s_step-end_infinite]" />
        </div>
        <div className="hidden shrink-0 sm:block">
          <span className="text-tangerine">aaron</span>
          <span className="text-white/40"> · api</span>
        </div>
      </div>
      <div className="border-t border-line px-5 py-3">
        <div className="flex justify-between text-white/50">
          <span>
            <span className="font-semibold text-violet">threads</span> recent
          </span>
          <span>3 open · 1 settled</span>
        </div>
        <div className="mt-1 text-white/30">› type to filter</div>
        <div className="mt-1">
          {THREADS.map((t) => (
            <div key={t.title} className={`flex gap-3 whitespace-nowrap ${t.settled ? "opacity-45" : ""}`}>
              <span className={t.glyphClass}>{t.glyph}</span>
              <span className="w-56 truncate font-semibold text-white/90">{t.title}</span>
              <span className="hidden w-24 truncate md:inline">
                {t.agents.map((a, i) => (
                  <span key={a.name} className={a.color}>
                    {i > 0 ? " " : ""}
                    {a.name}
                  </span>
                ))}
              </span>
              <span className="hidden w-16 truncate text-white/60 md:inline">{t.project}</span>
              <span className="ml-auto w-12 text-right text-white/60">{t.tokens}</span>
              <span className="hidden w-20 text-right text-white/40 sm:inline">{t.age}</span>
            </div>
          ))}
        </div>
      </div>
      <div className={`border-t border-line px-5 py-2 text-white/40 transition-opacity duration-500 ${done ? "opacity-100" : "opacity-0"}`}>
        <span className="text-violet">Enter</span> starts aaron on it, in place. <span className="text-violet">↓</span> picks up a thread instead.
      </div>
    </div>
  );
}

/* ---------------------------------------------------------------- sections */

function Section({
  eyebrow,
  title,
  children,
  aside,
}: {
  eyebrow: string;
  title: React.ReactNode;
  children: React.ReactNode;
  aside: React.ReactNode;
}) {
  return (
    <section className="relative mx-auto grid max-w-5xl items-center gap-12 px-6 py-24 md:grid-cols-2">
      <div>
        <p className="text-spectrum mb-3 inline-block text-sm font-semibold uppercase tracking-widest">{eyebrow}</p>
        <h2 className="mb-5 text-4xl font-semibold tracking-tight text-fg">
          {title}
        </h2>
        <div className="space-y-4 text-lg leading-relaxed text-muted">{children}</div>
      </div>
      <div>{aside}</div>
    </section>
  );
}

function Point({ icon: Icon, title, children }: { icon: React.ComponentType<{ className?: string }>; title: string; children: React.ReactNode }) {
  return (
    <div className="flex gap-3">
      <Icon className="mt-1 h-5 w-5 shrink-0 text-pink" />
      <p className="text-base text-muted">
        <span className="font-medium text-fg">{title}</span> {children}
      </p>
    </div>
  );
}

function Card({ children, className = "" }: { children: React.ReactNode; className?: string }) {
  return <div className={`rounded-2xl border border-line bg-surface p-6 font-mono text-[13px] leading-6 text-fg ${className}`}>{children}</div>;
}

function Lifecycle() {
  const steps = [
    { label: "open", note: "being worked on", dot: "bg-violet" },
    { label: "settled", note: "finished, still listed", dot: "bg-emerald-500" },
    { label: "archived", note: "put away", dot: "bg-dim" },
  ];
  return (
    <Card>
      <div className="mb-4 text-muted">thread · Add retries to the webhook sender</div>
      <div className="space-y-1 text-muted">
        <div>
          <span className="text-tangerine">aaron</span> · session 1 · laptop · <span className="text-dim">ended</span>
        </div>
        <div>
          <span className="text-sky">jude</span> · session 2 · laptop · <span className="text-dim">ended</span>
        </div>
        <div>
          <span className="text-tangerine">aaron</span> · session 3 · mini · <span className="text-pink">working</span>
        </div>
      </div>
      <div className="mt-6 flex flex-wrap items-center gap-2">
        {steps.map((s, i) => (
          <div key={s.label} className="flex items-center gap-2">
            {i > 0 && <ArrowRight className="h-4 w-4 text-dim" />}
            <div className="rounded-lg border border-line bg-ink px-3 py-2">
              <div className="flex items-center gap-2 font-semibold text-fg">
                <span className={`h-2 w-2 rounded-full ${s.dot}`} />
                {s.label}
              </div>
              <div className="text-xs text-muted">{s.note}</div>
            </div>
          </div>
        ))}
      </div>
    </Card>
  );
}

function Identity() {
  return (
    <Card className="space-y-4">
      <div>
        <div className="flex items-center gap-2 text-base font-semibold text-fg">
          <span className="h-2.5 w-2.5 rounded-full bg-orange-500" /> aaron
          <BadgeCheck className="h-4 w-4 text-pink" />
        </div>
        <div className="text-muted">ed25519 · 7Hq3…vK2p · attested by you</div>
      </div>
      <div className="grid grid-cols-[6rem_1fr] gap-y-1">
        <span className="text-dim">harness</span>
        <span>claude · opus</span>
        <span className="text-dim">mode</span>
        <span>autopilot</span>
        <span className="text-dim">may reach</span>
        <span>
          <span className="text-sky">jude</span>, <span className="text-sun">kit</span>
        </span>
        <span className="text-dim">budget</span>
        <span>$20 / day</span>
        <span className="text-dim">runs on</span>
        <span>mini</span>
      </div>
      <div className="rounded-lg border border-line bg-ink p-3">
        <div className="text-xs text-dim">Message from aaron, another Axel agent on this thread:</div>
        <div className="mt-1">
          <span className="font-semibold text-sky">@jude</span> can you review the retry backoff in src/webhooks.rs? Reply to aaron when done.
        </div>
      </div>
    </Card>
  );
}

function Hosts() {
  return (
    <Card>
      <div className="flex items-stretch justify-between gap-4">
        <div className="flex-1 rounded-lg border border-line bg-ink p-4">
          <Laptop className="mb-2 h-6 w-6 text-fg" />
          <div className="font-semibold text-fg">laptop</div>
          <div className="text-xs text-muted">aaron&apos;s key stays here</div>
        </div>
        <div className="flex flex-col items-center justify-center gap-1 text-xs text-dim">
          <span>thread + code →</span>
          <span>delegation →</span>
          <span>← events, branch</span>
        </div>
        <div className="flex-1 rounded-lg border border-pink/50 bg-ink p-4">
          <Server className="mb-2 h-6 w-6 text-pink" />
          <div className="font-semibold text-fg">mini</div>
          <div className="text-xs text-muted">signs for aaron: one thread, 7 days</div>
        </div>
      </div>
      <div className="mt-5 space-y-1 text-muted">
        <div>
          <span className="text-dim">$</span> axel agent edit aaron --host mini
        </div>
        <div>
          <span className="text-dim">$</span> axel session move 01j9… --to local
        </div>
      </div>
    </Card>
  );
}

function Guardrails() {
  const rows = [
    { mode: "autopilot", what: "never asks; the guard still refuses irreversible commands" },
    { mode: "edits", what: "file edits go through, everything else asks" },
    { mode: "supervised", what: "asks before every action" },
    { mode: "plan", what: "read-only" },
  ];
  return (
    <Card>
      {rows.map((r) => (
        <div key={r.mode} className="grid grid-cols-[7rem_1fr] gap-2 py-1">
          <span className="font-semibold text-fg">{r.mode}</span>
          <span className="text-muted">{r.what}</span>
        </div>
      ))}
      <div className="mt-4 border-t border-line pt-4 text-muted">
        <div>
          aaron · 41k tokens · <span className="text-fg">$0.62</span> today
        </div>
        <div className="text-dim">refused: jude has spent $20.00 today and reached its daily budget</div>
      </div>
    </Card>
  );
}

const HARNESSES = "Claude Code, Codex, OpenCode and any ACP agent";

/* ---------------------------------------------------------------- page */

export default function Home() {
  return (
    <div className="relative">
      <section className="relative z-10 flex min-h-screen flex-col items-center justify-center px-6 pb-16 pt-24 text-center">
        {/* Axel the axolotl. */}
        <img src="/mascot.webp" alt="Axel, a smiling axolotl in neon colours" width={984} height={827} className="mb-6 w-[min(20rem,70vw)] drop-shadow-[0_0_50px_rgba(247,69,198,0.45)]" />
        <h1 className="text-spectrum pb-2 text-7xl font-bold leading-none tracking-tight sm:text-8xl" style={{ fontFamily: "'Space Grotesk', sans-serif" }}>
          axel
        </h1>
        <p className="mt-6 max-w-2xl text-2xl font-medium text-fg">
          Named agents. Durable threads. <span className="text-spectrum">Your terminal.</span>
        </p>
        <p className="mt-4 max-w-xl text-lg text-muted">
          Type what you want done and press Enter. Your agents take it from there, on this machine or another, while you get on with your day.
        </p>
        <div className="mt-8 flex flex-wrap items-center justify-center gap-3">
          <a
            href={`${DOCS}/quick-start`}
            className="group inline-flex items-center gap-2 bg-spectrum rounded-full px-7 py-3 font-semibold text-white shadow-[0_0_40px_rgba(247,69,198,0.45)] transition hover:shadow-[0_0_60px_rgba(247,69,198,0.7)] hover:brightness-110"
          >
            Get started
            <ArrowRight className="h-4 w-4 transition-transform group-hover:translate-x-0.5" />
          </a>
          <a href={REPO} className="inline-flex items-center gap-2 rounded-full border border-line px-7 py-3 font-semibold text-white/80 transition hover:bg-ink/5 hover:text-white">
            View on GitHub
          </a>
        </div>
        <div className="relative mt-14 flex w-full justify-center">
          {/* The axolotl's spectrum behind the panes, blurred into a glow. */}
          <div aria-hidden className="bg-spectrum absolute -inset-x-10 -inset-y-12 mx-auto max-w-4xl rounded-[3rem] opacity-40 blur-3xl" />
          {/* A 1px spectrum outline: the panes glow like the mascot. */}
          <div className="bg-spectrum relative w-full max-w-3xl rounded-xl p-px shadow-[0_0_60px_rgba(166,77,255,0.35)]">
            <Launcher />
          </div>
        </div>
        <p className="mt-6 text-sm text-white/40">
          Runs {HARNESSES}, inside{" "}
          <a href="https://www.superlogical.com/rex" className="underline decoration-white/20 underline-offset-4 hover:text-white/70">
            Rex
          </a>
          .
        </p>
      </section>

      <div className="relative z-10 bg-ink">
        <div className="pointer-events-none absolute inset-0 mx-auto max-w-5xl">
          <div className="absolute bottom-0 left-0 top-0 w-px border-l border-dashed border-line" />
          <div className="absolute bottom-0 right-0 top-0 w-px border-r border-dashed border-line" />
        </div>

        <Section eyebrow="Threads" title="Work that outlives the terminal" aside={<Lifecycle />}>
          <p>A thread is what you asked for and everything done about it: the conversation, every tool call, the agents that joined, the projects it touches.</p>
          <p>
            Agents come and go. Panes close, laptops sleep. The thread stays, ready to read or resume. When it&apos;s finished, settle it: it stays listed until you archive it.
          </p>
          <div className="space-y-3 pt-2">
            <Point icon={GitBranch} title="In place or in a worktree.">
              Several projects at once, or none for a scratch idea.
            </Point>
            <Point icon={Terminal} title="Resume where it stopped.">
              A new session continues the agent&apos;s last conversation.
            </Point>
          </div>
        </Section>

        <Section eyebrow="Agents" title="Agents with names, keys and limits" aside={<Identity />}>
          <p>
            aaron is an Ed25519 identity you created and attested, with a harness, a model, skills, a colour, permissions and a budget. Everything he does is signed by him.
          </p>
          <div className="space-y-3 pt-2">
            <Point icon={MessagesSquare} title="They work together.">
              An agent can ask another on the same thread for a review or a hand, and get the answer as its next prompt. You see every message.
            </Point>
            <Point icon={KeyRound} title="Only who you allowed.">
              Agents start with no reach. Loops, runaway chains and overspending are refused, with the command that would allow it.
            </Point>
          </div>
        </Section>

        <Section eyebrow="Remote hosts" title="Ask, then close the laptop" aside={<Hosts />}>
          <p>
            Register a machine you reach over SSH, like a Mac mini at home. <code className="rounded bg-raised px-1.5 py-0.5 text-base">@aaron@mini</code> runs aaron there, or
            make the mini his default.
          </p>
          <p>
            The thread view streams as if he ran locally. Close the lid on the subway; his branch and uncommitted work come home when you reconnect. A running session can even move
            between machines and keep its conversation.
          </p>
          <div className="space-y-3 pt-2">
            <Point icon={ShieldCheck} title="Keys never leave home.">
              The host gets a short-lived grant to sign for aaron, on one thread only.
            </Point>
          </div>
        </Section>

        <Section eyebrow="Autopilot" title="No babysitting" aside={<Guardrails />}>
          <p>
            Each agent has a mode for routine permissions, a guard for the things that are never routine, and a daily budget. When an agent does need you, the question shows up in
            the thread and in your inbox.
          </p>
          <div className="space-y-3 pt-2">
            <Point icon={Coins} title="Every token counted.">
              Input, output and cache tokens, and the estimated cost, per thread and per agent.
            </Point>
            <Point icon={Terminal} title="No screen scraping.">
              Status comes from hooks, the Agent Client Protocol or <code className="rounded bg-raised px-1 py-0.5 text-sm">axel status</code>, and Rex&apos;s tab indicators
              agree.
            </Point>
          </div>
        </Section>

        <section className="relative mx-auto max-w-5xl border-t border-line px-6 py-24 text-center">
          <h2 className="text-4xl font-semibold tracking-tight text-fg">
            Why not today?
          </h2>
          <p className="mx-auto mt-4 max-w-xl text-lg text-muted">Build it, create an agent, and give it something to do.</p>
          <div className="mx-auto mt-8 max-w-xl rounded-2xl bg-surface p-6 text-left font-mono text-sm leading-7 text-white/80">
            <div>
              <span className="text-white/35">$</span> just install
            </div>
            <div>
              <span className="text-white/35">$</span> axel agent new aaron --harness claude
            </div>
            <div>
              <span className="text-white/35">$</span> axel
            </div>
          </div>
          <a href={`${DOCS}/installation`} className="mt-8 inline-flex items-center gap-2 font-semibold text-pink hover:text-sun">
            Read the installation guide <ArrowRight className="h-4 w-4" />
          </a>
        </section>
      </div>
    </div>
  );
}
