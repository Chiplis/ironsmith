import {readFile,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {createHash} from 'node:crypto';

// The pinned upstream updater uses wxDialUpManager even on wx builds where it
// is disabled. Clang also rejects passing LocalFileName to a variadic error
// formatter in the symbol editor. The CLI also needs process-local font
// registration: wx's macOS AddPrivateFont only validates bundle paths.
const [source]=process.argv.slice(2);
if(!source)throw new Error('Usage: patch-macos.mjs MSE_SOURCE');
const fragment='    wxDialUpManager* manager = wxDialUpManager::Create();\n    bool connected = manager->IsOk() && manager->IsOnline();\n    delete manager;\n    if (!connected) return;';
const replacement='#if wxUSE_DIALUP_MANAGER\n'+fragment+'\n#else\n    return; // Native reference build: updater connectivity API unavailable.\n#endif';
const hash=value=>createHash('sha256').update(value).digest('hex');
const fontFragment='#include <wx/font.h>';
const fontReplacement=[fontFragment,'#ifdef __APPLE__','#include <CoreText/CoreText.h>',
  'static bool registerReferenceFont(const wxString& filename) {',
  '  const wxScopedCharBuffer path = filename.utf8_str();',
  '  CFURLRef url = CFURLCreateFromFileSystemRepresentation(nullptr, reinterpret_cast<const UInt8*>(path.data()), path.length(), false);',
  '  if (!url) return false;',
  '  CFErrorRef error = nullptr;',
  '  bool ok = CTFontManagerRegisterFontsForURL(url, kCTFontManagerScopeProcess, &error);',
  '  if (error) {',
  '    if (CFErrorGetCode(error) == kCTFontManagerErrorAlreadyRegistered) ok = true;',
  '    if (!ok) wxLogError("Reference font registration failed (%ld): %s", long(CFErrorGetCode(error)), filename);',
  '    CFRelease(error);',
  '  }',
  '  CFRelease(url);',
  '  return ok;',
  '}',
  '#else',
  'static bool registerReferenceFont(const wxString& filename) { return wxFont::AddPrivateFont(filename); }',
  '#endif'].join('\r\n');
const patches=[
  {file:'src/gui/downloadable_installers.hpp',fragment,replacement,change:'Guard unavailable wxDialUpManager in updater'},
  {file:'src/gui/symbol/window.cpp',fragment:'_ERROR_1_("can\'t write symbol", new_filename)',replacement:'_ERROR_1_("can\'t write symbol", new_filename.toStringForKey())',change:'Convert LocalFileName to wxString for symbol-save error formatting'},
  {file:'src/main.cpp',fragment:'int MSE::OnRun() {\r\n  try {',replacement:'int MSE::OnRun() {\r\n  if (argc > 1) wxLog::SetActiveTarget(new wxLogStderr()); // Keep reference CLI diagnostics out of modal dialogs.\r\n  try {',change:'Route command-line wx logs to stderr instead of modal GUI dialogs'},
  {file:'src/data/font.cpp',fragment:fontFragment,replacement:fontReplacement,change:'Provide process-local CoreText font registration for the macOS CLI'},
  {file:'src/data/font.cpp',fragment:'fontFilePath.EndsWith(_(".ttf")) || fontFilePath.EndsWith(_(".otf"))',replacement:'fontFilePath.Lower().EndsWith(_(".ttf")) || fontFilePath.Lower().EndsWith(_(".otf"))',change:'Register packaged fonts with uppercase extensions, including MATRIX.TTF'},
  {file:'src/data/font.cpp',fragment:'wxFont::AddPrivateFont(fontFilePath)',replacement:'registerReferenceFont(fontFilePath)',change:'Use actual font registration instead of macOS wx bundle-path validation'},
];
const records=[];
for(const relative of new Set(patches.map(patch=>patch.file))){
  const file=resolve(source,relative),current=await readFile(file,'utf8');
  const group=patches.filter(patch=>patch.file===relative);
  let original=current;
  for(const patch of [...group].reverse())original=original.replace(patch.replacement,patch.fragment);
  let patched=original;
  for(const patch of group){
    if(patched.split(patch.fragment).length!==2)throw new Error(`Unexpected upstream source ${relative}; refusing to patch`);
    patched=patched.replace(patch.fragment,patch.replacement);
  }
  if(current!==patched)await writeFile(file,patched);
  records.push({file:relative,originalSha256:hash(original),patchedSha256:hash(patched),changes:group.map(patch=>patch.change)});
}
await writeFile(resolve(source,'native-reference-portability.json'),JSON.stringify({scope:'Build/CLI portability and process-local macOS font registration; layout and drawing algorithms unchanged',patches:records},null,2)+'\n');
console.log('Native build portability patches applied');
