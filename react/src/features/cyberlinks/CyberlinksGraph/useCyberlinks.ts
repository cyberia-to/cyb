import { useEffect, useMemo, useState } from 'react';
import { useBackend } from 'src/contexts/backend/backend';
import { Order_By as OrderBy, useCyberlinksByParticleQuery } from 'src/generated/graphql';

const _valueByKeyOrSelf = (key: string, obj: Record<string, any>) => obj[key] || key;

function useParticlesPreview() {
  const { senseApi, isDbInitialized } = useBackend();
  const [particlesPreview, setParticlesPreview] = useState<Record<string, string>>({});
  const [isLoading, setIsLoading] = useState<boolean>(true);

  useEffect(() => {
    if (senseApi && isDbInitialized) {
      (async () => {
        setIsLoading(true);
        const { rows } = await senseApi.getAllParticles(['cid', 'text', 'mime']);
        const particles = Object.fromEntries(
          rows.map((row) => {
            return [row[0], row[1] || row[2]];
          })
        );

        setParticlesPreview(particles);
        setIsLoading(false);
      })();
    }
  }, [isDbInitialized, senseApi]);

  return { particlesPreview, isLoading };
}
// TODO: moved, refactor and maybe delete
function useCyberlinks(
  { address }: { address?: string },
  {
    limit = 1024,
    skip,
  }: {
    limit?: number | false;
    skip?: boolean;
  }
) {
  let where;
  if (address) {
    where = { neuron: { _eq: address } };
  } else {
    where = {};
  }

  const {
    loading,
    error,
    data: gqlData,
  } = useCyberlinksByParticleQuery({
    variables: {
      where,
      orderBy: { height: OrderBy.Desc },
      limit: limit || undefined,
    },
    skip,
  });

  const cyberlinks = gqlData?.cyberlinks;
  const { isLoading, particlesPreview } = useParticlesPreview();

  const data = useMemo(() => {
    if (!cyberlinks) {
      return {
        nodes: [],
        links: [],
      };
    }

    // TODO: a lot of loops, try to refactor
    const from = cyberlinks.map((a) => a.from);
    const to = cyberlinks.map((a) => a.to);

    const object = Array.from(new Set(from.concat(to))).map((value) => ({
      // id: valueByKeyOrSelf(value, particlesPreview),
      id: value,
    }));

    const links = [];

    for (let i = 0; i < cyberlinks.length; i++) {
      links[i] = {
        // source: valueByKeyOrSelf(cyberlinks[i].from, particlesPreview),
        // target: valueByKeyOrSelf(cyberlinks[i].to, particlesPreview),
        source: cyberlinks[i].from,
        target: cyberlinks[i].to,
        name: cyberlinks[i].transaction_hash,
        subject: cyberlinks[i].neuron,
        // curvative: getRandomInt(20, 500) / 1000,
      };
    }

    return {
      nodes: object,
      links,
    };
  }, [cyberlinks]);

  return {
    data,
    loading,
  };
}

export default useCyberlinks;
