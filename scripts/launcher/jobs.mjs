import {spawn} from 'node:child_process';
import {randomUUID} from 'node:crypto';

export class Jobs {
  jobs = [];
  start(name, executable, args, cwd, timeout = 600000) {
    if (this.jobs.some(job => job.code === null)) throw new Error('A task is already running');
    const child = spawn(executable, args, {cwd, shell:false, stdio:['ignore','pipe','pipe']});
    const job = {id:randomUUID(), name, log:'', code:null, started:new Date().toISOString(), child};
    this.jobs.unshift(job); this.jobs.length = Math.min(30, this.jobs.length);
    const append = chunk => {job.log = (job.log + chunk.toString()).slice(-131072);};
    child.stdout.on('data', append); child.stderr.on('data', append);
    job.done = new Promise(resolve => {
      let timer;
      if (timeout) timer = setTimeout(() => {append('\nTask timed out'); this.cancel(job.id);}, timeout);
      child.on('error', error => append(error.message));
      child.on('close', (code, signal) => {clearTimeout(timer); job.code = code ?? -1; job.signal = signal; resolve();});
    });
    return job;
  }
  cancel(id) { const job = this.jobs.find(job => job.id === id); if (!job || job.code !== null) return; job.child.kill('SIGTERM'); const timer = setTimeout(() => {if (job.code === null) job.child.kill('SIGKILL');}, 2000); timer.unref(); }
  stop() {for (const job of this.jobs) this.cancel(job.id);}
  public(job) {return {id:job.id, name:job.name, log:job.log, code:job.code, started:job.started};}
  list() {return this.jobs.map(job => this.public(job));}
}
