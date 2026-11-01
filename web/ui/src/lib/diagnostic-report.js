// Reports are data only; imported journal methods are never executed.
export function parseDiagnosticReport(text) {
  let report;
  try { report = JSON.parse(text); } catch { throw new Error('This file is not valid JSON.'); }
  if (!report || typeof report !== 'object' || Array.isArray(report)
    || !Array.isArray(report.traces) || !Array.isArray(report.events)
    || !report.mainThread || typeof report.mainThread !== 'object') {
    throw new Error('This file is not an Ironsmith diagnostic report.');
  }
  return report;
}
