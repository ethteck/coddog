import { createRootRoute, Link, Outlet } from '@tanstack/react-router';
import logo from '../assets/coddoglogo.svg';

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
      <img src={logo} width="100" alt="coddog logo" className="logo" />
      <div className="p-2 flex gap-2">
        <Link to="/" className="[&.active]:font-bold">
          Home
        </Link>{' '}
        <Link
          to="/symbol"
          search={{ name: '' }}
          className="[&.active]:font-bold"
        >
          Symbol
        </Link>{' '}
        <Link to="/admin" className="[&.active]:font-bold">
          Admin
        </Link>{' '}
      </div>
      <hr />
      <Outlet />
      <hr />
      <div className="p-2 flex gap-2 footer">
        <p>coddog - the dog that sniffs for cod</p>
      </div>
      {/*<TanStackRouterDevtools/>*/}
    </>
  ),
});
