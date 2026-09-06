import http from 'node:http';
import {createReadStream} from 'node:fs';
import {stat,realpath} from 'node:fs/promises';
import path from 'node:path';
const root=await realpath(process.argv[2] || 'target/web-demo');
const port=Number(process.argv[3] || 4173);
const mime={'.html':'text/html; charset=utf-8','.js':'text/javascript','.json':'application/json','.css':'text/css','.wasm':'application/wasm','.png':'image/png','.wav':'audio/wav','.ogg':'audio/ogg','.mp4':'video/mp4','.webm':'video/webm'};
http.createServer(async(req,res)=>{
  try{
    let pathname=decodeURIComponent(new URL(req.url,'http://localhost').pathname);
    if(pathname.endsWith('/'))pathname+='index.html';
    const file=await realpath(path.join(root,pathname));
    if(!file.startsWith(root+path.sep))throw new Error('outside root');
    const metadata=await stat(file);if(!metadata.isFile())throw new Error('not a file');
    const headers={'Content-Type':mime[path.extname(file)] || 'application/octet-stream','Accept-Ranges':'bytes','Cache-Control':'no-cache'};
    const range=req.headers.range?.match(/^bytes=(\d+)-(\d*)$/);
    if(range){const start=Number(range[1]),end=range[2]?Math.min(Number(range[2]),metadata.size-1):metadata.size-1;
      if(start>end||start>=metadata.size){res.writeHead(416,{'Content-Range':`bytes */${metadata.size}`});res.end();return;}
      res.writeHead(206,{...headers,'Content-Length':end-start+1,'Content-Range':`bytes ${start}-${end}/${metadata.size}`});createReadStream(file,{start,end}).pipe(res);
    }else{res.writeHead(200,{...headers,'Content-Length':metadata.size});createReadStream(file).pipe(res);}
  }catch{res.writeHead(404);res.end('Not found');}
}).listen(port,'127.0.0.1',()=>console.log(`RenRS: http://127.0.0.1:${port}/`));
