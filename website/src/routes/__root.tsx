import {createRootRoute, Link, Outlet,} from '@tanstack/react-router';

export const Route = createRootRoute({
    head: () => ({
        meta: [
            {
                title: 'coddog',
            },
        ],
    }),
    component: () => (
        <>
            <h2>coddog</h2>
            <div className="p-2 flex gap-2">
                <Link to="/" className="[&.active]:font-bold">
                    Home
                </Link>{' '}
                <Link to="/match" search={{"name": ""}} className="[&.active]:font-bold">
                    Match
                </Link>{' '}
                <Link to="/submatch" search={{"name": ""}} className="[&.active]:font-bold">
                    Submatch
                </Link>{' '}
                <Link to="/admin" className="[&.active]:font-bold">
                    Admin
                </Link>{' '}
            </div>
            <hr/>
            <Outlet/>
            <hr/>
            <div className="p-2 flex gap-2 footer">
                <p>coddog - the dog that sniffs for cod</p>
            </div>
            {/*<TanStackRouterDevtools/>*/}
        </>
    ),
});
