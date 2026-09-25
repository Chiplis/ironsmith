import useUiText from "@/i18n/useUiText";
import { useState } from "react";
import { useGame } from "@/context/GameContext";
import { copyTextToClipboard } from "@/lib/clipboard";
import {
  createOrganizedTournament,
  issueTournamentInvite,
  listOrganizedTournaments,
  listTournamentCertificates,
  previewTournamentInvite,
} from "@/lib/tournament/credentials";

const inputClass = "fantasy-field w-full px-3 py-2 text-[14px] text-foreground outline-none";
const labelClass = "grid gap-1 text-[12px] uppercase tracking-[0.18em] text-muted-foreground";
const panelClass = "lobby-sheet-panel fantasy-sheet-section grid gap-4 p-4";
const buttonClass = "stone-pill inline-flex items-center justify-center rounded-none px-4 py-2 text-[13px] font-semibold uppercase tracking-[0.18em] transition-all disabled:cursor-not-allowed disabled:opacity-50";
const headingClass = "text-[13px] font-semibold uppercase tracking-[0.18em] text-foreground";
const noteClass = "text-[13px] leading-5 text-muted-foreground";

const formatDate = (ms) => new Date(Number(ms || 0)).toLocaleDateString();

// Players redeem organizer invites here; organizers create tournaments and
// issue one invite per player. Matches themselves are created from the
// Create tab with the "Tournament match" connection.
export default function TournamentPanel() {
  const ui = useUiText();
  const { redeemTournamentInvite, setStatus } = useGame();
  const [certificates, setCertificates] = useState(() => listTournamentCertificates());
  const [organized, setOrganized] = useState(() => listOrganizedTournaments());
  const [inviteCode, setInviteCode] = useState("");
  const [preview, setPreview] = useState(null);
  const [tournamentName, setTournamentName] = useState("");
  const [inviteNames, setInviteNames] = useState({});
  const [busy, setBusy] = useState(false);

  const run = async (task, success) => {
    setBusy(true);
    try {
      await task();
      if (success) setStatus(success);
    } catch (error) {
      setStatus(error?.message || String(error), true);
    } finally {
      setBusy(false);
    }
  };

  const handleCodeChange = async (value) => {
    setInviteCode(value);
    try { setPreview(value.trim() ? await previewTournamentInvite(value) : null); }
    catch { setPreview(null); }
  };

  const handleRedeem = () => run(async () => {
    await redeemTournamentInvite(inviteCode);
    setCertificates(listTournamentCertificates());
    setInviteCode("");
    setPreview(null);
  }, "Invite redeemed. Create or join a tournament match from the other tabs.");

  const handleCreateTournament = () => run(async () => {
    await createOrganizedTournament(tournamentName);
    setOrganized(listOrganizedTournaments());
    setTournamentName("");
  }, "Tournament created");

  const handleIssue = (tournamentId) => run(async () => {
    await issueTournamentInvite(tournamentId, inviteNames[tournamentId]);
    setOrganized(listOrganizedTournaments());
    setInviteNames((prev) => ({ ...prev, [tournamentId]: "" }));
  }, "Invite issued. Send the code privately to that player.");

  const handleCopy = async (code) => {
    const copied = await copyTextToClipboard(code);
    setStatus(copied ? "Copied invite code" : "Could not copy invite code", !copied);
  };

  return (
    <div className="lobby-sheet-tournaments grid gap-4 md:grid-cols-2">
      <section className={panelClass}>
        <h3 className={headingClass}>{ui("Your tournaments")}</h3>
        <p className={noteClass}>{ui("Paste the invite code your organizer sent you. Redeeming ties the seat to this browser, so use the browser you will play from.")}</p>
        <label className={labelClass}>{ui("Invite code")}
          <textarea className={`${inputClass} min-h-[84px] font-mono text-[12px]`} value={inviteCode}
            onChange={(event) => handleCodeChange(event.target.value)} placeholder="IST1." />
        </label>
        {preview && <p className={noteClass}>{ui("Invite for") + " "}<strong>{preview.playerName}</strong>{" · " + preview.tournamentName + " · " + ui("expires") + " " + formatDate(preview.expiresAt)}</p>}
        <button type="button" className={buttonClass} disabled={busy || !preview} onClick={handleRedeem}>{ui("Redeem invite")}</button>
        {certificates.length > 0 && <ul className="grid gap-2">
          {certificates.map((entry) => (
            <li key={entry.tournamentId} className={noteClass}>
              <strong className="text-foreground">{entry.tournamentName}</strong>{" · " + ui("playing as") + " " + entry.playerName + " · " + ui("until") + " " + formatDate(entry.expiresAt)}
            </li>
          ))}
        </ul>}
      </section>
      <section className={panelClass}>
        <h3 className={headingClass}>{ui("Organize")}</h3>
        <p className={noteClass}>{ui("The tournament's signing key stays in this browser. Issue one invite per player; whoever redeems a code first owns that seat.")}</p>
        <div className="flex gap-2">
          <input className={inputClass} value={tournamentName} placeholder={ui("Tournament name")}
            onChange={(event) => setTournamentName(event.target.value)} />
          <button type="button" className={buttonClass} disabled={busy || !tournamentName.trim()} onClick={handleCreateTournament}>{ui("Create")}</button>
        </div>
        {organized.map((tournament) => (
          <div key={tournament.tournamentId} className="grid gap-2">
            <div className={headingClass}>{tournament.name}</div>
            <div className="flex gap-2">
              <input className={inputClass} value={inviteNames[tournament.tournamentId] || ""} placeholder={ui("Player name")}
                onChange={(event) => setInviteNames((prev) => ({ ...prev, [tournament.tournamentId]: event.target.value }))} />
              <button type="button" className={buttonClass} disabled={busy || !String(inviteNames[tournament.tournamentId] || "").trim()}
                onClick={() => handleIssue(tournament.tournamentId)}>{ui("Issue invite")}</button>
            </div>
            {tournament.invites.map((invite) => (
              <div key={invite.inviteId} className="flex items-center gap-2">
                <span className={`${noteClass} min-w-[8ch]`}>{invite.playerName}</span>
                <input className={`${inputClass} font-mono text-[11px]`} readOnly value={invite.code} onFocus={(event) => event.target.select()} />
                <button type="button" className={buttonClass} onClick={() => handleCopy(invite.code)}>{ui("Copy")}</button>
              </div>
            ))}
          </div>
        ))}
      </section>
    </div>
  );
}
