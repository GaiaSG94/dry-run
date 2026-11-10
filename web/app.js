// Logique front-end de Dry Run : aucun CDN, aucune dépendance externe.
// Tous les montants sont manipulés en BigInt à partir de chaînes décimales
// (yoctoNEAR) : aucun flottant, aucune perte de précision.

const YOCTO_PER_NEAR = 1000000000000000000000000n; // 10^24
let lastPayload = null; // dernière demande, pour la confirmation d'un WARN

document.addEventListener("DOMContentLoaded", () => {
    initTabs();
    initTransferForm();
    initCallForm();
    initFreezeButton();
    initRefreshButton();
    initTerminalToggle();
    initConfirmButton();
    refreshAll();
    setInterval(fetchStatus, 5000);
});

/* ---------- Utilitaires de montants ---------- */

/** Convertit un montant NEAR décimal saisi en yoctoNEAR (chaîne décimale). */
function toYocto(nearText) {
    const [whole = "0", frac = ""] = String(nearText).trim().split(".");
    const wholePart = BigInt(whole === "" ? "0" : whole);
    const fracPart = BigInt((frac + "0".repeat(24)).slice(0, 24));
    return (wholePart * YOCTO_PER_NEAR + fracPart).toString();
}

/** Formate des yoctoNEAR (chaîne ou nombre) en NEAR lisibles. */
function formatYocto(yoctoText) {
    try {
        const value = BigInt(String(yoctoText));
        const sign = value < 0n ? "-" : "";
        const abs = value < 0n ? -value : value;
        const whole = abs / YOCTO_PER_NEAR;
        const frac = (abs % YOCTO_PER_NEAR).toString().padStart(24, "0").slice(0, 6);
        return `${sign}${whole}.${frac}`;
    } catch (err) {
        return String(yoctoText);
    }
}

/** Échappe le texte inséré dans le DOM. */
function esc(text) {
    const div = document.createElement("div");
    div.textContent = String(text);
    return div.innerHTML;
}

/* ---------- État du coffre ---------- */

async function fetchStatus() {
    try {
        const res = await fetch("/api/status");
        if (!res.ok) return;
        const data = await res.json();
        document.getElementById("stat-account").textContent = data.account_id;
        document.getElementById("stat-balance").textContent = formatYocto(data.balance) + " NEAR";
        document.getElementById("stat-tx-cap").textContent = formatYocto(data.tx_cap) + " NEAR";
        document.getElementById("stat-daily-cap").textContent = formatYocto(data.daily_cap) + " NEAR";
        document.getElementById("stat-spent").textContent = formatYocto(data.spent_24h) + " NEAR";
        document.getElementById("stat-owner").textContent = data.owner;

        const badge = document.getElementById("vault-badge");
        badge.textContent = data.is_frozen ? "GELÉ" : "OUVERT";
        badge.className = "status-badge " + (data.is_frozen ? "badge-frozen" : "badge-active");
    } catch (err) {
        console.error("état du coffre indisponible :", err);
    }
}

/* ---------- Frise des demandes ---------- */

async function fetchLog() {
    try {
        const res = await fetch("/api/log");
        if (!res.ok) return;
        const entries = await res.json();
        const container = document.getElementById("timeline-list");

        if (!entries.length) {
            container.innerHTML =
                '<div class="timeline-empty">Journal vide. Les demandes simulées apparaîtront ici.</div>';
            return;
        }

        container.innerHTML = "";
        [...entries].reverse().forEach((entry) => container.appendChild(timelineRow(entry)));
    } catch (err) {
        console.error("frise indisponible :", err);
    }
}

/** Construit une ligne de frise pour une entrée du journal. */
function timelineRow(entry) {
    const verdict = String(entry.verdict).toLowerCase();
    const row = document.createElement("div");
    row.className = `timeline-entry timeline-entry-${verdict}`;
    const rules = entry.rule_ids && entry.rule_ids.length ? entry.rule_ids.join(", ") : "aucune";

    row.innerHTML = `
        <div>
            <strong>#${entry.id} — ${esc(entry.intent_summary || "sans intention")}</strong>
            <div class="timeline-meta">${esc(entry.summary)} • règles : ${esc(rules)}</div>
        </div>
        <div class="timeline-side">
            <span class="verdict-tag verdict-${verdict}">${esc(entry.verdict)}</span>
            <div class="timeline-meta">${entry.executed ? "signé et exécuté" : "rien signé"}</div>
        </div>`;
    return row;
}

/* ---------- Formulaires ---------- */

function initTabs() {
    const tabs = document.querySelectorAll(".tab-btn");
    const forms = {
        transfer: document.getElementById("form-transfer"),
        call: document.getElementById("form-call"),
    };
    tabs.forEach((tab) => {
        tab.addEventListener("click", () => {
            tabs.forEach((t) => t.classList.remove("active"));
            tab.classList.add("active");
            const target = tab.getAttribute("data-tab");
            Object.entries(forms).forEach(([name, form]) => {
                form.classList.toggle("hidden", name !== target);
            });
        });
    });
}

function initTransferForm() {
    document.getElementById("form-transfer").addEventListener("submit", async (event) => {
        event.preventDefault();
        const to = document.getElementById("tf-to").value.trim();
        const amount = toYocto(document.getElementById("tf-amount").value);
        const summary = document.getElementById("tf-intent").value.trim();
        await propose({
            kind: "transfer",
            to,
            amount,
            intent: { summary, beneficiaries: [to], min_in: [] },
        });
    });
}

function initCallForm() {
    document.getElementById("form-call").addEventListener("submit", async (event) => {
        event.preventDefault();
        const contract = document.getElementById("call-contract").value.trim();
        const method = document.getElementById("call-method").value.trim();
        const deposit = toYocto(document.getElementById("call-deposit").value);
        const summary = document.getElementById("call-intent").value.trim();
        const beneficiary = document.getElementById("call-beneficiary").value.trim();
        const minInRaw = document.getElementById("call-min-in").value.trim();

        const args = readArgs();
        if (args === null) return;

        const min_in = [];
        if (minInRaw.includes(":")) {
            const [token, amountText] = minInRaw.split(":");
            min_in.push([token.trim(), toYocto(amountText)]);
        }

        await propose({
            kind: "function_call",
            contract,
            method,
            args,
            deposit,
            gas: 30000000000000,
            intent: { summary, beneficiaries: beneficiary ? [beneficiary] : [], min_in },
        });
    });
}

/** Lit les arguments JSON du formulaire d'appel ; `null` si invalides. */
function readArgs() {
    const raw = document.getElementById("call-args").value.trim();
    if (!raw) return {};
    try {
        return JSON.parse(raw);
    } catch (err) {
        alert("Arguments JSON invalides : " + err.message);
        return null;
    }
}

/* ---------- Proposition et carte des effets ---------- */

async function propose(payload, confirmed = false) {
    try {
        const body = Object.assign({}, payload, { confirm: confirmed });
        const res = await fetch("/api/propose", {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(body),
        });
        if (!res.ok) {
            const err = await res.json();
            alert("Demande refusée par le serveur : " + (err.error || res.statusText));
            return;
        }
        lastPayload = payload;
        renderCard(await res.json());
        refreshAll();
    } catch (err) {
        alert("Erreur réseau : " + err);
    }
}

/** Affiche la carte des effets : verdict, soldes, règles, reçus. */
function renderCard(outcome) {
    const view = outcome.card_view;
    const verdict = String(outcome.verdict).toLowerCase();

    document.getElementById("card-placeholder").classList.add("hidden");
    document.getElementById("card-content").classList.remove("hidden");

    const tag = document.getElementById("verdict-tag");
    tag.textContent = outcome.verdict;
    tag.className = `verdict-tag verdict-${verdict}`;

    document.getElementById("card-summary").textContent = view.summary;
    document.getElementById("card-exec-status").innerHTML = outcome.executed
        ? '<span class="ok">signé et exécuté</span>'
        : '<span class="ko">rien signé</span>';

    renderBalances(view);
    renderRules(view);

    const receipts = view.receipts;
    document.getElementById("card-receipts").textContent =
        `${receipts.total} reçu(s), ${receipts.failed} échec(s), ` +
        `fonds coincés : ${receipts.stranded ? "oui" : "non"} — gas ${view.gas_burnt}`;

    document.getElementById("card-terminal-raw").textContent = outcome.card;

    const confirmBtn = document.getElementById("btn-confirm-warn");
    confirmBtn.classList.toggle("hidden", verdict !== "warn");
}

function renderBalances(view) {
    const box = document.getElementById("card-balances");
    box.innerHTML = "";
    const movements = [...(view.near_deltas || []), ...(view.ft_deltas || [])];

    if (!movements.length) {
        box.textContent = "aucun mouvement de solde";
        return;
    }

    movements.forEach((move) => {
        const isNear = move.delta_yocto !== undefined;
        const raw = isNear ? move.delta_yocto : move.delta;
        const negative = String(raw).startsWith("-");
        const row = document.createElement("div");
        row.className = "delta-row";
        row.innerHTML = `
            <span>${esc(move.account)}</span>
            <span class="${negative ? "delta-negative" : "delta-positive"}">
                ${esc(isNear ? formatYocto(raw) + " NEAR" : raw + " " + move.token)}
            </span>`;
        box.appendChild(row);
    });
}

function renderRules(view) {
    const box = document.getElementById("card-rules");
    box.innerHTML = "";

    if (!view.rules || !view.rules.length) {
        box.innerHTML = '<span class="ok">aucune règle enfreinte : les effets suivent l\'intention</span>';
        return;
    }
    view.rules.forEach((rule) => {
        const severity = String(rule.severity).toUpperCase();
        const row = document.createElement("div");
        row.className = "rule-hit-row " + (severity === "BLOCK" ? "rule-hit-block" : "rule-hit-warn");
        row.innerHTML = `<strong>${esc(rule.id)} ${esc(rule.title)}</strong>
            <div class="rule-detail">${esc(rule.detail)}</div>`;
        box.appendChild(row);
    });
}

/* ---------- Actions : gel, confirmation, rafraîchissement ---------- */

function initFreezeButton() {
    document.getElementById("freeze-btn").addEventListener("click", async () => {
        if (!confirm("Geler le coffre maintenant ? Toute demande ultérieure sera refusée.")) return;
        try {
            const res = await fetch("/api/freeze", { method: "POST" });
            if (!res.ok) {
                alert("Le gel a échoué.");
                return;
            }
            refreshAll();
        } catch (err) {
            alert("Erreur réseau pendant le gel : " + err);
        }
    });
}

function initConfirmButton() {
    const button = document.getElementById("btn-confirm-warn");
    button.addEventListener("click", () => {
        if (!lastPayload) return;
        if (!confirm("Confirmer cette demande malgré l'avertissement du pare-feu ?")) return;
        propose(lastPayload, true);
    });
}

function initRefreshButton() {
    document.getElementById("btn-refresh-log").addEventListener("click", refreshAll);
}

function initTerminalToggle() {
    const button = document.getElementById("toggle-terminal-card");
    button.addEventListener("click", () => {
        document.getElementById("card-terminal-raw").classList.toggle("hidden");
    });
}

function refreshAll() {
    fetchStatus();
    fetchLog();
}
