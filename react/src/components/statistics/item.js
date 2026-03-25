import { Icon, Pane } from '@cybercongress/gravity';
import { ContainerGradientText } from '../containerGradient/ContainerGradient';
import Tooltip from '../tooltip/tooltip';
import styles from './styles.module.scss';

// const iconHelp = require('../../image/_ionicons_svg_ios-help-circle-outline.svg');

export function ContainerCard({ children, col }) {
  return (
    <div
      style={{ gridTemplateColumns: `repeat(${col}, 1fr)`, ...styles }}
      className={styles.containerStatistics}
    >
      {children}
    </div>
  );
}

export function Card({ title, value, tooltipValue, positionTooltip, stylesContainer }) {
  return (
    <div style={{ ...stylesContainer }} className={styles.card}>
      <span className="card-title">{value}</span>
      <span className="card-value">
        {title}{' '}
        {tooltipValue && (
          <Pane marginLeft={5}>
            <Tooltip placement={positionTooltip} tooltip={tooltipValue}>
              <Icon icon="info-sign" color="#3ab793d4" />
            </Tooltip>
          </Pane>
        )}
      </span>
    </div>
  );
}

export function CardStatisics({
  title,
  value,
  link,
  styleContainer,
  styleValue,
  styleTitle,
  tooltipValue,
  positionTooltip,
  status = 'green',
}) {
  return (
    <ContainerGradientText status={status} userStyleContent={styleContainer}>
      <div style={styleValue} className={styles.containerValue}>
        {(value || value === 0) && <div>{value}</div>}
      </div>

      {title && (
        <div className={styles.containerTextLink}>
          <div style={styleTitle} className={styles.containerText}>
            {title}
            {tooltipValue && (
              <div className={styles.containerIcon}>
                <Tooltip placement={positionTooltip} tooltip={tooltipValue}>
                  <Icon icon="info-sign" color="#3ab793d4" />
                </Tooltip>
              </div>
            )}
          </div>
          {link && <Icon icon="arrow-right" color="#4ed6ae" marginLeft={5} />}
        </div>
      )}
    </ContainerGradientText>
  );
}
