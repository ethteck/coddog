import { Link, Outlet, createRootRoute } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools';

export const Route = createRootRoute({
  component: () => (
    <>
      <div className="p-2 flex gap-2">
        <Link to="/" className="[&.active]:font-bold">
          Home
        </Link>{' '}
        <Link to="/projects" className="[&.active]:font-bold">
          Projects
        </Link>{' '}
        <Link to="/match" className="[&.active]:font-bold">
          Match
        </Link>{' '}
      </div>
      <hr />
      <Outlet />
      <hr />
      <p>coddog - the dog that sniffs for cod</p>

      <TanStackRouterDevtools />
    </>
  ),
});
