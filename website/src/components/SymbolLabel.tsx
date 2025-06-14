export function SymbolLabel({name, project_name, object_name}: {
    name: string,
    project_name: string,
    object_name: string
}) {
    return (
        <>
            <b>{name}</b> - {project_name} ({object_name})
        </>
    );
}