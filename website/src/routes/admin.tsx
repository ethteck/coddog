import { useQuery } from '@tanstack/react-query';
import { createFileRoute, Link } from '@tanstack/react-router';
import { fetchProjects } from '../api/projects.tsx';

export const Route = createFileRoute('/admin')({
  head: () => ({
    meta: [
      {
        title: 'coddog - admin',
      },
    ],
  }),
  component: Admin,
});

function Admin() {
  const { isPending, isError, data, error } = useQuery({
    queryKey: ['projects'],
    queryFn: fetchProjects,
  });

  if (isPending) {
    return <div>Loading...</div>;
  }

  if (isError) {
    return (
      <div>
        Error: {error.name} {error.message}
      </div>
    );
  }

  return (
    <div className="projects">
      {data.map((project) => (
        <div className="project" key={project.id}>
          <h2>{project.name}</h2>
          <p>Platform: {project.platform}</p>
          {project.repo && (
            <p>
              Repo:{' '}
              <Link to={project.repo} target="_blank" rel="noopener noreferrer">
                {project.repo}
              </Link>
            </p>
          )}
        </div>
      ))}
    </div>
  );
}
