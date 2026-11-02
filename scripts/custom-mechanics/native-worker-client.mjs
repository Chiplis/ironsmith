import {spawn} from 'node:child_process';
import {createInterface} from 'node:readline';
export class NativeWorker {
 constructor(){this.start();}
 start(){const proc=spawn(process.env.AUDIT_NATIVE_WORKER,[],{stdio:['pipe','pipe','pipe']});this.proc=proc;proc.stdin.on('error',()=>{});proc.stderr.on('data',()=>{});
  createInterface({input:proc.stdout}).on('line',line=>{if(proc!==this.proc||!this.pending)return;const p=this.pending;this.pending=null;clearTimeout(p.timer);try{const result=JSON.parse(line);for(const f of result.faces||[])if(f.status==='compiled'&&(f.compiler_facts?.enforceFilterClaims!=='true'||f.compiler_facts?.enforceLineClaims!=='true'||f.compiler_facts?.allowUnsupported!=='false'))throw Error('AUDIT CONFIGURATION ERROR: strict compiler facts missing');if(result.fatal){proc.kill('SIGKILL');this.start();}p.resolve(result);}catch(e){p.reject(e);}});
  proc.on('exit',()=>{if(proc!==this.proc)return;if(this.pending){clearTimeout(this.pending.timer);this.pending.reject(Error('compiler subprocess exited'));this.pending=null;}if(!this.stopping)this.start();});
 }
 run(input){return new Promise((resolve,reject)=>{const timer=setTimeout(()=>{this.pending=null;this.proc.kill('SIGKILL');this.start();reject(Error(`compile timeout after ${Number(process.env.AUDIT_TIMEOUT_MS||120000)/1000} seconds`));},Number(process.env.AUDIT_TIMEOUT_MS||120000));this.pending={resolve,reject,timer};this.proc.stdin.write(JSON.stringify(input)+'\n');});}
 stop(){this.stopping=true;this.proc.stdin.end();}
}
