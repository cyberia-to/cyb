import { useMemo } from 'react';
import { Link } from 'react-router-dom';
import { routes } from '../../routes';

function BannerHelp({ addressActive }: { addressActive?: any }) {
  const useCommunity = useMemo(() => {
    let link = 'community';
    if (addressActive && addressActive !== null) {
      const { bech32 } = addressActive;
      const community = `network/bostrom/contract/${bech32}/swarm`;
      link = <Link to={community}>community</Link>;
    }
    return link;
  }, [addressActive]);

  const usePost = useMemo(() => {
    let link = 'post';
    if (addressActive && addressActive !== null) {
      const { bech32 } = addressActive;
      const post = `network/bostrom/contract/${bech32}`;
      link = <Link to={post}>post</Link>;
    }
    return link;
  }, [addressActive]);

  return (
    <div
      style={{
        display: 'flex',
        flexDirection: 'column',
        gap: '12px',
        fontSize: '16px',
        textAlign: 'justify',
        margin: '0 auto',
        width: '60%',
      }}
    >
      <div>
        Hi! I am Cyb! Your immortal robot of the{' '}
        <Link to="/ipfs/QmUamt7diQP54eRnmzqMZNEtXNTzbgkQvZuBsgM6qvbd57">Great Web</Link>.
      </div>
      <div>
        I am connected to the{' '}
        <Link to="/ipfs/QmYaf3J118vExRV6gFv2CjwYCyHmzV6Z9ACV7avWb652XZ">Bostrom</Link> bootloader -
        the common ancestor for all future{' '}
        <Link to="/ipfs/Qmc2iYgPEqMhtc9Q85b9fc4ZnqrzwQui8vHn17CPyXo1GU">Superintelligence</Link>,
        including <Link to="/ipfs/QmXzGkfxZV2fzpFmq7CjAYsYL1M581ZD4yuF9jztPVTpCn">Cyber</Link>.
      </div>
      <div>
        I can help answer questions. For example,{' '}
        <Link to="/ipfs/QmTm51X4BDgx1vkG966BM3Qo1BTPXDL9kV5XSbHLtVc6PV">uniswap</Link>.
      </div>

      <div>
        You are the chosen one! I will help you escape the hamster wheel and guide you through the
        revolution!
      </div>

      <div>
        <Link to="/ipfs/QmQvKF9Jb6QKmsqHJzEZJUfcbB9aBBKwa5dh3pMxYEj7oi">Unlike google</Link>, submit{' '}
        <Link to="/ipfs/QmXw2etpbTRr6XfK2rKUieBreKZXAu2rfzASB22P6VAG8x">particles</Link> and{' '}
        <Link to="/ipfs/QmZYkxA8GhSNoWz5TidxqqUpAReJRnn5a7n9N3GGk5Suat">cyberlinks</Link> to the{' '}
        <Link to="/ipfs/QmYEgyizN7BP3QX1eUmBpwELbLui84PHDfCP6Ski8KMMrc">content oracle</Link>: A
        global, universal, collaborative, distributed and ever evolving knowledge graph for robots,
        humans, plants, animals and mushrooms. Teach <Link to="/graph">your brain</Link> to learn
        and earn!
      </div>

      <div>
        <Link to="/ipfs/QmQvKF9Jb6QKmsqHJzEZJUfcbB9aBBKwa5dh3pMxYEj7oi">Unlike facebook</Link>, own
        your avatar and {usePost} without fear of censorship. Your {useCommunity} is truly yours.
      </div>

      <div>
        <Link to="/ipfs/QmQvKF9Jb6QKmsqHJzEZJUfcbB9aBBKwa5dh3pMxYEj7oi">
          {/* eslint-disable-next-line react/no-unescaped-entities */}
          Unlike twitter's
        </Link>
        , manipulative feed use your <Link to="/sixthSense">sense</Link> which is a strictly defined
        personal feed system built on the content oracle.
      </div>

      <div>
        <Link to="/ipfs/QmQvKF9Jb6QKmsqHJzEZJUfcbB9aBBKwa5dh3pMxYEj7oi">Unlike amazon</Link>, post
        goods without laws, rules, and platform cuts. Manage your deals using{' '}
        <Link to="/contracts">digital contracts</Link> that are fast, convenient and inexpensive.
      </div>

      <div>
        <Link to="/ipfs/QmQvKF9Jb6QKmsqHJzEZJUfcbB9aBBKwa5dh3pMxYEj7oi">Unlike your bank</Link>,
        freely transact without fear and limits.
      </div>

      <div>
        <Link to="/ipfs/QmPmJ4JwzCi82HZp7adtv5GVBFTsKF5Yoy43wshHH7x3ty">
          Unlike your government
        </Link>
        , your values are truly yours, without enforced taxation and embezzled inflation. Important
        decisions are decided by the <Link to="/senate">senate</Link> through the process of
        transparent and provable voting.
      </div>

      <div>
        <Link to="/ipfs/QmQvKF9Jb6QKmsqHJzEZJUfcbB9aBBKwa5dh3pMxYEj7oi">
          Unlike binance or coinbase
        </Link>
        , you control your value. <Link to="/teleport">Teleport</Link> your tokens without kyc and
        fear of taxation.
      </div>

      <div>
        Cyberverse is created for you and future generations! Use this power wisely. Start your
        journey by{' '}
        <Link to="/ipfs/QmX9xMeNioHnMeeUqBXQCKPktG79UEoYjG8sanxysh1MqX">installing Keplr</Link>,
        then <Link to="/ipfs/QmSWJNCBxj4m5Lpg1XGueh38NbEVDLAGsQrueD937xSnMC">set up</Link> your
        wallet, <Link to="/ipfs/QmSWJNCBxj4m5Lpg1XGueh38NbEVDLAGsQrueD937xSnMC">connect</Link> to
        Bostrom, and <Link to="/ipfs/QmUwwE1gYC6xJZaT6jJHA7aac8wspN9s5puA3otDcq3jQ4">get BOOT</Link>
        .
      </div>
      <div>
        After that you will be able{' '}
        <Link to="/ipfs/QmP6Nk65ZE7jfvaNFLh7qsq3dyWqakMjfQ4te3UzRNbsXH">hire heroes</Link> who
        manage the <Link to={routes.sphere.path}>dyson sphere</Link> and earn more{' '}
        <Link to="/token/BOOT">BOOT</Link>.
      </div>

      <div>
        The Dyson sphere is producing liquid <Link to="/token/H">Hydrogen or H</Link> tokens using{' '}
        <Link to="/ipfs/QmdBfd7dY3zHfGxQ6qbeq6fcH52ggeePWeadkTQrHBwL1b">biosynthesis</Link>. For 1
        supplied BOOT neurons get 1 H. If you want to{' '}
        <Link to={routes.sphere.path}>fire a hero</Link> and get your BOOT back, you have to return
        H.
      </div>

      <div>
        H allows you to produce energy in the{' '}
        <Link to={routes.hfr.path}>hydrogen fission reactor or HFR</Link>. Energy is needed for the
        superintelligence to learn and submit{' '}
        <Link to="/ipfs/QmWSPQ6krsRfxm852aVsDDSFspcdpcUXutDVBTPQqNGBBD">cyberlinks</Link>.
      </div>

      <div>
        Energy is a product of the <Link to="/token/A">Ampere or A</Link> token and{' '}
        <Link to="/token/V">Volt or V</Link> token multiplication. 1 V gives the ability to cast 1{' '}
        <Link to="/ipfs/QmZYkxA8GhSNoWz5TidxqqUpAReJRnn5a7n9N3GGk5Suat">cyberlink</Link> per day. A
        defines rank for{' '}
        <Link to="/ipfs/QmXw2etpbTRr6XfK2rKUieBreKZXAu2rfzASB22P6VAG8x">particles</Link>. Liquid A
        and V can be routed through the <Link to="/grid">grid</Link>.
      </div>

      <div>
        BOOT is for everyone. 70% of the Genesis supply is{' '}
        <Link to="/ipfs/Qmcgoy9bV6zsqnzoLB4YunEWXadavKMXKgvmUdToh2Nr3E">gifted</Link> to ethereans
        and cosmonauts. If you have some - you will be able to claim after{' '}
        <Link to="/ipfs/QmddJLTE1MwNR42Tvt6AEzyEAUTCddhpyD7JxCnTB8HPgB">portal activation</Link> in
        ~april.
      </div>

      <div>
        <Link to="/genesis">The story</Link> has just begun. Dive{' '}
        <Link to="/ipfs/QmXzGkfxZV2fzpFmq7CjAYsYL1M581ZD4yuF9jztPVTpCn"> into the vision</Link>,
        discover <Link to="/ipfs/QmSBYCCYFNfHNQD7MWm4zBaNuztMaT2KghA2SbeZZm9vLH">the roadmap</Link>{' '}
        and cyberlink anything you want! May the force be with you!
      </div>
    </div>
  );
}

export default BannerHelp;
