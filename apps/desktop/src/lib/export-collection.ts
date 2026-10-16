import {exportCollection} from "@/lib/tauri-api";
export async function exportCollectionToFile(collectionPath:string,collectionName:string,format:"postman"):Promise<void>{
    const result=await exportCollection(collectionPath,format);
    const {save}=await import("@tauri-apps/plugin-dialog");
    const path=await save({defaultPath:`${collectionName}.postman_collection.json`,filters:[{name:"Postman Collection v2.1",extensions:["json"]}]});
    if(path){const {writeTextFile}=await import("@tauri-apps/plugin-fs");await writeTextFile(path,result);}
}
