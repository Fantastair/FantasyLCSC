const project = await eda.dmt_Project.getCurrentProjectInfo();
return {
  client: eda.sys_Environment.isClient(),
  online: eda.sys_Environment.isOnlineMode(),
  version: eda.sys_Environment.getEditorCurrentVersion(),
  project: project ? {
    uuid: project.uuid,
    name: project.friendlyName || project.name,
    data: project.data,
  } : null,
};
