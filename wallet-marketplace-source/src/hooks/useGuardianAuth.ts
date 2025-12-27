import { useEffect, useState } from 'react';
import axios from 'axios';

interface GuardianStatus {
  is_guardian_creator: boolean;
  creator_name?: string;
  creator_title?: string;
}

/**
 * Hook to check if the current user is the Guardian creator.
 * 
 * Use this in the navbar or any component that needs to conditionally
 * show the Guardian Control Room link.
 * 
 * @example
 * ```tsx
 * const { isGuardianCreator, loading } = useGuardianAuth();
 * 
 * {isGuardianCreator && (
 *   <NavLink to="/guardian">Guardian Control Room</NavLink>
 * )}
 * ```
 */
export function useGuardianAuth() {
  const [isGuardianCreator, setIsGuardianCreator] = useState(false);
  const [creatorName, setCreatorName] = useState<string | null>(null);
  const [creatorTitle, setCreatorTitle] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const checkStatus = async () => {
      try {
        const { data } = await axios.get<GuardianStatus>('/me/guardian');
        
        if (cancelled) return;

        setIsGuardianCreator(data.is_guardian_creator);
        setCreatorName(data.creator_name ?? null);
        setCreatorTitle(data.creator_title ?? null);
        setLoading(false);
      } catch (err) {
        if (cancelled) return;

        console.error('Failed to check guardian status:', err);
        setError(err instanceof Error ? err.message : 'Unknown error');
        setIsGuardianCreator(false);
        setLoading(false);
      }
    };

    checkStatus();

    return () => {
      cancelled = true;
    };
  }, []);

  return {
    isGuardianCreator,
    creatorName,
    creatorTitle,
    loading,
    error,
  };
}
