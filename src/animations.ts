// Animation data - loaded as raw text for wasm parsing
import jumpingJacksAnim from './assets/animations/jumping_jacks.json?raw';
import lungesAnim from './assets/animations/lunges.json?raw';
import squatJumpsAnim from './assets/animations/squat_jumps.json?raw';
import parachuteJumpAnim from './assets/animations/parachute_jump.json?raw';
import abCrunchAnim from './assets/animations/ab_crunch.json?raw';
import backExtensionAnim from './assets/animations/back_extension.json?raw';
import handLegTouchJumpAnim from './assets/animations/hand_leg_touch_jump.json?raw';
import pushUpsAnim from './assets/animations/push_ups.json?raw';
import sideMuscleAnim from './assets/animations/side_muscle.json?raw';
import feetForwardJumpAnim from './assets/animations/feet_forward_jump.json?raw';
import straightLegLeanAnim from './assets/animations/straight_leg_lean.json?raw';
import backwardsLungeAnim from './assets/animations/backwards_lunge.json?raw';
import oneLegJumpAnim from './assets/animations/one_leg_jump.json?raw';
import crossedArmsSitupAnim from './assets/animations/crossed_arms_situp.json?raw';
import acdcScissorJumpAnim from './assets/animations/acdc_scissor_jump.json?raw';
import easierLegLeanAnim from './assets/animations/easier_leg_lean.json?raw';
import legLiftRunningAnim from './assets/animations/leg_lift_running.json?raw';
import oneLegSquatsAnim from './assets/animations/one_leg_squats.json?raw';
import plankingAnim from './assets/animations/planking.json?raw';
import burpeesAnim from './assets/animations/burpees.json?raw';

// Animation name to JSON map (for keyframe editor)
// Map exercise names (from workout JSON) to their animation data
export const animationMap: Map<string, string> = new Map([
    ['Jumping Jacks', jumpingJacksAnim],
    ['Lunges', lungesAnim],
    ['Squat Jumps', squatJumpsAnim],
    ['Parachute Jump', parachuteJumpAnim],
    ['Ab Crunch', abCrunchAnim],
    ['Back Extension', backExtensionAnim],
    ['Hand-leg Touch Jump', handLegTouchJumpAnim],
    ['Push-Ups', pushUpsAnim],
    ['Side muscle', sideMuscleAnim],
    ['Feet Forward Jump', feetForwardJumpAnim],
    ['Straight Leg Lean', straightLegLeanAnim],
    ['Backwards Lunge', backwardsLungeAnim],
    ['One Leg Jump', oneLegJumpAnim],
    ['Crossed-Arms Situp', crossedArmsSitupAnim],
    ['AC-DC/Scrissor Jump', acdcScissorJumpAnim],
    ['Easier Leg Lean', easierLegLeanAnim],
    ['Leg Lift Running', legLiftRunningAnim],
    ['One Leg Squats', oneLegSquatsAnim],
    ['Planking', plankingAnim],
    ['Burpees', burpeesAnim],
]);
