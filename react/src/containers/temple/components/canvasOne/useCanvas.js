import { useEffect, useRef } from 'react';

const sphereRad = 250;
const radiusSp = 0.8;
let displayWidth;
let displayHeight;
let timer;
let wait;
let count;
let numToAddEachFrame;
let particleList;
let recycleBin;
let particleAlpha;
let r;
let g;
let b;
let fLen;
let m;
let projCenterX;
let projCenterY;
let zMax;
let turnAngle;
let turnSpeed;
let sphereCenterY;
let sphereCenterZ;
let particleRad;
let zeroAlphaDepth;
let randAccelX;
let randAccelY;
let randAccelZ;
let gravity;
let rgbString;
// we are defining a lot of variables used in the screen update functions globally so that they don't have to be redefined every frame.
let p;
let outsideTest;
let nextParticle;
let sinAngle;
let cosAngle;
let rotX;
let rotZ;
let depthAlphaFactor;
let i;
let theta;
let phi;
let x0;
let y0;
let z0;

function useCanvas() {
  const canvasRef = useRef(null);

  const draw = (ctx) => {
    ctx.fillStyle = `${rgbString + depthAlphaFactor * p.alpha})`;
    // draw
    ctx.beginPath();
    ctx.arc(p.projX, p.projY, m * particleRad, 0, 2 * Math.PI, false);
    ctx.closePath();
    ctx.fill();
  };

  useEffect(() => {
    const theCanvas = canvasRef.current;
    const context = theCanvas.getContext('2d');
    function canvasApp() {
      // const theCanvas = document.getElementById('canvasOne');

      function addParticle(x0Temp, y0Temp, z0Temp, vx0, vy0, vz0) {
        let newParticle;
        // check recycle bin for available drop:
        if (recycleBin.first != null) {
          newParticle = recycleBin.first;
          // remove from bin
          if (newParticle.next != null) {
            recycleBin.first = newParticle.next;
            newParticle.next.prev = null;
          } else {
            recycleBin.first = null;
          }
        }
        // if the recycle bin is empty, create a new particle (a new ampty object):
        else {
          newParticle = {};
        }
        // add to beginning of particle list
        if (particleList.first == null) {
          particleList.first = newParticle;
          newParticle.prev = null;
          newParticle.next = null;
        } else {
          newParticle.next = particleList.first;
          particleList.first.prev = newParticle;
          particleList.first = newParticle;
          newParticle.prev = null;
        }
        // initialize
        newParticle.x = x0Temp;
        newParticle.y = y0Temp;
        newParticle.z = z0Temp;
        newParticle.velX = vx0;
        newParticle.velY = vy0;
        newParticle.velZ = vz0;
        newParticle.age = 0;
        newParticle.dead = false;
        if (Math.random() < 0.5) {
          newParticle.right = true;
        } else {
          newParticle.right = false;
        }
        return newParticle;
      }

      function recycle(_p) {
        // remove from particleList
        if (particleList.first === _p) {
          if (_p.next != null) {
            p.next.prev = null;
            particleList.first = _p.next;
          } else {
            particleList.first = null;
          }
        } else if (_p.next == null) {
          p.prev.next = null;
        } else {
          p.prev.next = _p.next;
          p.next.prev = _p.prev;
        }
        // add to recycle bin
        if (recycleBin.first == null) {
          recycleBin.first = _p;
          p.prev = null;
          p.next = null;
        } else {
          p.next = recycleBin.first;
          recycleBin.first.prev = _p;
          recycleBin.first = _p;
          p.prev = null;
        }
      }

      function onTimer() {
        // if enough time has elapsed, we will add new particles.
        count += 1;
        if (count >= wait) {
          count = 0;
          for (i = 0; i < numToAddEachFrame; i++) {
            theta = Math.random() * 2 * Math.PI;
            phi = Math.acos(Math.random() * 2 - 1);
            x0 = sphereRad * Math.sin(phi) * Math.cos(theta);
            y0 = sphereRad * Math.sin(phi) * Math.sin(theta);
            z0 = sphereRad * Math.cos(phi);
            // We use the addParticle function to add a new particle. The parameters set the position and velocity components.
            // Note that the velocity parameters will cause the particle to initially fly outwards away from the sphere center (after
            // it becomes unstuck).
            p = addParticle(
              x0,
              sphereCenterY + y0,
              sphereCenterZ + z0,
              0.002 * x0,
              0.002 * y0,
              0.002 * z0
            );
            // we set some "envelope" parameters which will control the evolving alpha of the particles.
            p.attack = 50;
            p.hold = 50;
            p.decay = 100;
            p.initValue = 0;
            p.holdValue = particleAlpha;
            p.lastValue = 0;
            // the particle will be stuck in one place until this time has elapsed:
            p.stuckTime = 90 + Math.random() * 20;
            p.accelX = 0;
            p.accelY = gravity;
            p.accelZ = 0;
          }
        }
        // update viewing angle
        turnAngle = (turnAngle + turnSpeed) % (2 * Math.PI);
        sinAngle = Math.sin(turnAngle);
        cosAngle = Math.cos(turnAngle);
        // background fill
        context.fillStyle = '#000000';
        context.fillRect(0, 0, displayWidth, displayHeight);
        // update and draw particles
        p = particleList.first;
        while (p != null) {
          // before list is altered record next particle
          nextParticle = p.next;
          // update age
          p.age += 1;
          // if the particle is past its "stuck" time, it will begin to move.
          if (p.age > p.stuckTime) {
            p.velX += p.accelX + randAccelX * (Math.random() * 2 - 1);
            p.velY += p.accelY + randAccelY * (Math.random() * 2 - 1);
            p.velZ += p.accelZ + randAccelZ * (Math.random() * 2 - 1);
            p.x += p.velX;
            p.y += p.velY;
            p.z += p.velZ;
          }
          /*
    			We are doing two things here to calculate display coordinates.
    			The whole display is being rotated around a vertical axis, so we first calculate rotated coordinates for
    			x and z (but the y coordinate will not change).
    			Then, we take the new coordinates (rotX, y, rotZ), and project these onto the 2D view plane.
    			*/
          rotX = cosAngle * p.x + sinAngle * (p.z - sphereCenterZ);
          rotZ = -sinAngle * p.x + cosAngle * (p.z - sphereCenterZ) + sphereCenterZ;
          m = (radiusSp * fLen) / (fLen - rotZ);
          p.projX = rotX * m + projCenterX;
          p.projY = p.y * m + projCenterY;
          // update alpha according to envelope parameters.
          if (p.age < p.attack + p.hold + p.decay) {
            if (p.age < p.attack) {
              p.alpha = ((p.holdValue - p.initValue) / p.attack) * p.age + p.initValue;
            } else if (p.age < p.attack + p.hold) {
              p.alpha = p.holdValue;
            } else if (p.age < p.attack + p.hold + p.decay) {
              p.alpha =
                ((p.lastValue - p.holdValue) / p.decay) * (p.age - p.attack - p.hold) + p.holdValue;
            }
          } else {
            p.dead = true;
          }
          // see if the particle is still within the viewable range.
          if (
            p.projX > displayWidth ||
            p.projX < 0 ||
            p.projY < 0 ||
            p.projY > displayHeight ||
            rotZ > zMax
          ) {
            outsideTest = true;
          } else {
            outsideTest = false;
          }
          if (outsideTest || p.dead) {
            recycle(p);
          } else {
            // depth-dependent darkening
            depthAlphaFactor = 1 - rotZ / zeroAlphaDepth;
            depthAlphaFactor =
              // eslint-disable-next-line no-nested-ternary
              depthAlphaFactor > 1 ? 1 : depthAlphaFactor < 0 ? 0 : depthAlphaFactor;

            draw(context);
          }
          p = nextParticle;
        }
      }
      function init() {
        wait = 1;
        count = wait - 1;
        numToAddEachFrame = 8;
        // particle color
        r = 70;
        g = 255;
        b = 140;
        rgbString = `rgba(${r},${g},${b},`; // partial string for color which will be completed by appending alpha value.
        particleAlpha = 1; // maximum alpha
        displayWidth = theCanvas.width;
        displayHeight = theCanvas.height;
        fLen = 320; // represents the distance from the viewer to z=0 depth.
        // projection center coordinates sets location of origin
        projCenterX = displayWidth / 2;
        projCenterY = displayHeight / 2;
        // we will not draw coordinates if they have too large of a z-coordinate (which means they are very close to the observer).
        zMax = fLen - 2;
        particleList = {};
        recycleBin = {};
        // random acceleration factors - causes some random motion
        randAccelX = 0.1;
        randAccelY = 0.1;
        randAccelZ = 0.1;
        gravity = -0; // try changing to a positive number (not too large, for example 0.3), or negative for floating upwards.
        particleRad = 2.5;
        sphereCenterY = 0;
        sphereCenterZ = -3 - sphereRad;
        // alpha values will lessen as particles move further back, causing depth-based darkening:
        zeroAlphaDepth = -750;
        turnSpeed = (2 * Math.PI) / 1600; // the sphere will rotate at this speed (one complete rotation every 1600 frames).
        turnAngle = 0; // initial angle
        timer = setInterval(onTimer, 54);
      }
      init();
    }
    canvasApp();

    return () => {
      clearInterval(timer);
    };
  }, [draw]);

  return { canvasRef };
}

export default useCanvas;
