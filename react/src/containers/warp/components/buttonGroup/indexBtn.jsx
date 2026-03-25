import stylesBtn from './stylesBtn.scss';

const classNames = require('classnames');

function ButtonTeleport({ status, children, isSelected, ...props }) {
  return (
    <button
      {...props}
      type="button"
      className={classNames(stylesBtn.teleportBtn, {
        [stylesBtn.teleportBtnCenter]: status === 'center',
        [stylesBtn.teleportBtnRight]: status === 'right',
        [stylesBtn.teleportBtnLeft]: status === 'left',
        [stylesBtn.teleportBtnSelected]: isSelected,
        [stylesBtn.teleportBtnDefault]: !isSelected,
      })}
    >
      <div
        className={classNames(stylesBtn.lampMenu, {
          [stylesBtn.teleportBtnSelected]: isSelected,
          [stylesBtn.teleportBtnDefault]: !isSelected,
          [stylesBtn.lampMenuCenter]: status === 'center',
          [stylesBtn.lampMenuCenterLeft]: status === 'center',
          // state for RIGHT menu button
          [stylesBtn.lampMenuRight]: status === 'right',
          // state for LEFT Lamp when menu button -> RIGHT
          [stylesBtn.lampMenuRightLeft]: status === 'right',
          // state for LEFT menu button
          [stylesBtn.lampMenuLeft]: status === 'left',
          // state for LEFT Lamp when menu button -> LEFT
          [stylesBtn.lampMenuLeftLeft]: status === 'left',
        })}
      />
      <div
        className={classNames(stylesBtn.lampMenu, {
          [stylesBtn.teleportBtnSelected]: isSelected,
          [stylesBtn.teleportBtnDefault]: !isSelected,
          [stylesBtn.lampMenuCenter]: status === 'center',
          [stylesBtn.lampMenuCenterRight]: status === 'center',
          // state for RIGHT menu button
          [stylesBtn.lampMenuRight]: status === 'right',
          // state for RIGHT Lamp when menu button -> RIGHT
          [stylesBtn.lampMenuRightRight]: status === 'right',
          // state for LEFT menu button
          [stylesBtn.lampMenuLeft]: status === 'left',
          // state for RIGHT Lamp when menu button -> LEFT
          [stylesBtn.lampMenuLeftRight]: status === 'left',
        })}
      />
      {children}
    </button>
  );
}

export default ButtonTeleport;
