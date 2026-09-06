import {spawnSync} from 'node:child_process';
import {mkdir,cp,writeFile} from 'node:fs/promises';
import path from 'node:path';
const root=path.resolve(process.argv[2] || 'target/media-fixture');
const ffmpeg=process.env.RENRS_FFMPEG || path.resolve('target/media-tools/node_modules/@ffmpeg-installer/darwin-arm64/ffmpeg');
function run(command,args,env=process.env){const result=spawnSync(command,args,{stdio:'inherit',env});if(result.error)throw result.error;if(result.status!==0)throw new Error(`${command} failed`);}
await mkdir(root,{recursive:true});
await cp('demo/images',path.join(root,'images'),{recursive:true});
run(ffmpeg,['-nostdin','-v','error','-f','lavfi','-i','testsrc2=size=640x360:rate=24','-t','2','-c:v','libx264','-pix_fmt','yuv420p','-n',path.join(root,'input.mp4')]);
run('./target/debug/renrs-video',[path.join(root,'input.mp4'),root,'clips/intro'],{...process.env,RENRS_FFMPEG:ffmpeg});
await writeFile(path.join(root,'script.rns'),`config title "RenRS Media Test"
config id "org.renrs.media-test"
default name = "Reader"
label start:
    scene "images/studio.png"
    show "images/mira.png" as mira at center
    "Media checkpoint."
    timeline:
        @id "media.move" transform mira x 100 over 0.4 ease in_out
        @id "media.alpha" transform mira alpha 0.5 over 0.4
    scene "images/rooftop.png"
    @id "media.dissolve" transition dissolve 0.5
    @id "media.video" video "clips/intro/clip.json" over 2
    "Video finished."
    menu:
        "Finish":
            jump ending
        "Replay":
            jump start
label ending:
    "Thank you, [name]."
    return
`);
await writeFile(path.join(root,'resources.json'),JSON.stringify({exclude:['input.mp4']}));
await writeFile(path.join(root,'progress.json'),JSON.stringify({achievements:[{id:'media',title:'Media complete',label:'ending'}],gallery:[{id:'roof',title:'Rooftop',label:'ending',image:'images/rooftop.png'}],endings:[{id:'ending',title:'Completed',label:'ending'}],rollback_barriers:['ending']}));
console.log(root);
