import { useQuery } from '@tanstack/react-query';
import { createFileRoute } from '@tanstack/react-router';
import { zodValidator } from '@tanstack/zod-adapter';
import { z } from 'zod';
import { fetchSymbolAsm, fetchSymbolMetadata } from '../api/symbols';
import { DualAssemblyViewer } from '../components/DualAssemblyViewer';

// Define search schema with proper types and defaults

const searchSchema = z
  .object({
    sym1: z
      .string()
      .length(5)
      .regex(/^[a-zA-Z0-9]+$/),
    start1: z.number().gte(0),
    sym2: z
      .string()
      .length(5)
      .regex(/^[a-zA-Z0-9]+$/),
    start2: z.number().gte(0),
    len: z.number().gt(0),
  })
  .refine(
    (data) => {
      // Ensure sym1 and sym2 are not the same
      return data.sym1 !== data.sym2;
    },
    {
      message: 'Cannot compare a symbol to itself',
    },
  );

export const Route = createFileRoute('/compare')({
  component: ComparePage,
  validateSearch: zodValidator(searchSchema),
});

function ComparePage() {
  const search = Route.useSearch();

  const {
    data: sym1,
    isLoading: isLoadingSym1,
    isError: isErrorSym1,
    error: sym1Error,
  } = useQuery({
    queryKey: ['metadata', search.sym1],
    queryFn: () => fetchSymbolMetadata(search.sym1),
    staleTime: 5 * 60 * 1000,
  });

  const {
    data: sym2,
    isLoading: isLoadingSym2,
    isError: isErrorSym2,
    error: sym2Error,
  } = useQuery({
    queryKey: ['metadata', search.sym2],
    queryFn: () => fetchSymbolMetadata(search.sym2),
    staleTime: 5 * 60 * 1000,
  });

  const {
    data: sym1Asm,
    isLoading: isLoadingAsm1,
    isError: isErrorAsm1,
    error: errorAsm1,
  } = useQuery({
    queryKey: ['asm', search.sym1],
    queryFn: () => fetchSymbolAsm(search.sym1),
    staleTime: 5 * 60 * 1000,
  });

  const {
    data: sym2Asm,
    isLoading: isLoadingAsm2,
    isError: isErrorAsm2,
    error: errorAsm2,
  } = useQuery({
    queryKey: ['asm', search.sym2],
    queryFn: () => fetchSymbolAsm(search.sym2),
    staleTime: 5 * 60 * 1000,
  });

  if (isLoadingSym1 || isLoadingSym2) {
    return <div className="loading">Loading metadata...</div>;
  }

  if (isLoadingAsm1 || isLoadingAsm2) {
    return <div className="loading">Loading assembly data...</div>;
  }

  if (isErrorAsm1) {
    return <div className="error">{(errorAsm1 as Error).message}</div>;
  }
  if (isErrorAsm2) {
    return <div className="error">{(errorAsm2 as Error).message}</div>;
  }
  if (isErrorSym1) {
    return <div className="error">{(sym1Error as Error).message}</div>;
  }
  if (isErrorSym2) {
    return <div className="error">{(sym2Error as Error).message}</div>;
  }

  if (!sym1Asm || !sym2Asm) {
    return <div className="error">Assembly data could not be loaded</div>;
  }

  if (!sym1 || !sym2) {
    return <div className="error">Symbol data could not be loaded</div>;
  }

  return (
    <div>
      <h1>Symbol Comparison</h1>
      <DualAssemblyViewer
        leftAsm={sym1Asm}
        rightAsm={sym2Asm}
        leftStartLine={search.start1}
        rightStartLine={search.start2}
        leftMetadata={sym1}
        rightMetadata={sym2}
        maxDisplayLines={search.len}
      />
    </div>
  );
}
