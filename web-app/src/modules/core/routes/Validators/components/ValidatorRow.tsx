import { FC } from 'react';
import clsx from 'clsx';
import AccountAddress from '../../../../ui/AccountAddress';
import Money from '../../../../ui/Money';
import { IValidator } from '../../../../interface/Validator.interface';
import {
  CheckIcon,
  XMarkIcon,
  CheckCircleIcon,
  ExclamationCircleIcon,
  XCircleIcon,
} from '@heroicons/react/20/solid';
// import ProgressBar from './ProgressBar';
import Vouches from './Vouch/Vouches';

// Define icons for each status
const statusIcons = {
  accessible: <CheckCircleIcon className="h-5 w-5 text-green-500" />,
  notAccessible: <ExclamationCircleIcon className="h-5 w-5 text-yellow-500" />,
  invalidAddress: <XCircleIcon className="h-5 w-5 text-red-500" />,
};

interface ValidatorRowProps {
  validator: IValidator;
}

const ValidatorRow: FC<ValidatorRowProps> = ({ validator }) => {
  return (
    <tr className={clsx('whitespace-nowrap text-sm text-[#141414] text-center')}>
      <td className="px-2 md:px-4 lg:px-6 py-4">
        <AccountAddress address={validator.address} />
      </td>
      <td className="px-2 md:px-4 lg:px-6 py-4">{validator.handle}</td>
      {validator.inSet && (
        <td className="px-2 md:px-4 lg:px-6 py-4 text-center">
          {validator.grade.compliant ? (
            <CheckIcon className="w-5 h-5 text-green-500 inline" style={{ marginTop: '-3px' }} />
          ) : (
            <XMarkIcon className="w-5 h-5 text-red-500 inline" style={{ marginTop: '-3px' }} />
          )}
          <span className={validator.grade.failedBlocks > 0 ? 'text-red-500' : ''}>
            {validator.grade.failedBlocks.toLocaleString()}
          </span>{' '}
          / {validator.grade.proposedBlocks.toLocaleString()}
        </td>
      )}
      <td className="px-2 md:px-4 lg:px-6 py-4 text-center">
        {validator.auditQualification?.toLocaleString() ? (
          <XMarkIcon className="w-5 h-5 text-red-500 inline" style={{ marginTop: '-3px' }} />
        ) : (
          <CheckIcon className="w-5 h-5 text-green-500 inline" style={{ marginTop: '-3px' }} />
        )}
        {validator.auditQualification?.toLocaleString()}
      </td>
      <td className="px-2 md:px-4 lg:px-6 py-4 text-center">
        <Vouches vouches={validator.vouches} />
      </td>
      <td className="px-2 md:px-4 lg:px-6 py-4 text-right">
        {validator.currentBid
          ? `${formatPercentage(validator.currentBid.currentBid)} (${
              validator.currentBid.expirationEpoch > 10000
                ? '> 10,000'
                : validator.currentBid.expirationEpoch.toLocaleString()
            })`
          : ''}
      </td>
      {validator.inSet && (
        <td className="px-2 md:px-4 lg:px-6 py-4 text-center">
          {statusIcons[validator.vfnStatus]}
        </td>
      )}
      <td className="px-2 md:px-4 lg:px-6 py-4 text-right">
        <Money>{Number(validator.unlocked)}</Money>
      </td>
      <td className="px-2 md:px-4 lg:px-6 py-4 text-right">
        <Money>{Number(validator.balance)}</Money>
      </td>
      {validator.inSet && (
        <td className="px-2 md:px-4 lg:px-6 py-4 text-left">
          {validator.city ? `${validator.city}, ${validator.country}` : 'Unknown'}
        </td>
      )}
    </tr>
  );
};

function formatPercentage(value: number) {
  return (value / 1000).toLocaleString(undefined, { style: 'percent', minimumFractionDigits: 1 });
}
export default ValidatorRow;
