/**
 * Tells when the orbit controls may advance their damping.
 *
 * `OrbitControls.update()` spends 5 % of the rotation left by the last drag
 * each time it is called, and stops asking for frames once a step falls under
 * its threshold, leaving a little unspent. Calling it on frames drawn for
 * something else (the clock, an animation) would spend that remainder, so the
 * camera would creep. Only frames that follow an interaction call it, until it
 * reports that it has settled.
 */
export class CameraSettling {
  private settling = false;

  /** The controls' `start` and `change` events: the user moved the camera. */
  mark = (): void => {
    this.settling = true;
  };

  /** Runs `update` (the controls' update) if the camera is settling; true while it still moves. */
  step(update: () => boolean): boolean {
    const moving = this.settling && update();
    if (!moving) this.settling = false;
    return moving;
  }
}
