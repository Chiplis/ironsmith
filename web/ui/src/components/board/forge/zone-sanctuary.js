import * as THREE from 'three';

// Geometry is measured from the same DOM that places the cards: each player's
// shrine follows their seat, including mobile opponent switching and spectators.
export function createZoneSanctuary(scene, own) {
  const zones = new Map();
  const box = own(new THREE.BoxGeometry(1, 1, 1));
  const plane = own(new THREE.PlaneGeometry(1, 1));
  const stone = own(new THREE.MeshStandardMaterial({ color: 0x344354, roughness: .94, metalness: .08 }));
  const dark = own(new THREE.MeshStandardMaterial({ color: 0x111c2c, roughness: 1 }));
  const silver = own(new THREE.MeshStandardMaterial({ color: 0x7f9cba, roughness: .6, metalness: .55 }));
  const rift = own(new THREE.ShaderMaterial({
    transparent: true, depthWrite: false,
    uniforms: { time: { value: 0 } },
    vertexShader: 'varying vec2 p;void main(){p=uv*2.-1.;gl_Position=projectionMatrix*modelViewMatrix*vec4(position,1.);}',
    fragmentShader: `varying vec2 p;uniform float time;void main(){float edge=exp(-pow((p.x+.08*sin(p.y*13.))*18.,2.));float fade=1.-smoothstep(.55,1.,abs(p.y));gl_FragColor=vec4(.32,.48,.95,edge*fade*(.28+.04*sin(time)));}`,
  }));
  function make(type) {
    const group = new THREE.Group();
    function mesh(geometry, material, x, y, z, w, h, depth = 1) {
      const m = new THREE.Mesh(geometry, material);
      m.castShadow = material === stone || material === silver;
      m.receiveShadow = true;
      m.position.set(x, y, z); m.scale.set(w, h, depth); group.add(m); return m;
    }
    if (type === 'exile') {
      // A narrow fissure in the stone, without a circular zone outline.
      mesh(plane, rift, .46, 0, 1, .28, 1.06);
    } else if (type === 'command' || type === 'stack') {
      mesh(box, stone, 0, 0, -2, 1.02, 1.02, 4);
    } else if (type === 'graveyard') {
      mesh(box, stone, 0, 0, -2, 1.05, 1.05, 5);
      mesh(box, dark, 0, 0, 2, .87, .87, 2);
      mesh(box, stone, 0, .5, 4, .55, .12, 9);
      mesh(box, silver, 0, .5, 9, .16, .025, 1);
    } else if (type === 'library') {
      mesh(box, dark, 0, -.04, -4, 1.12, 1.12, 7);
      mesh(box, stone, 0, 0, 1, 1.03, 1.03, 6);
      mesh(box, silver, 0, -.48, 5, .83, .025, 1);
    } else if (type === 'hand') {
      mesh(box, stone, 0, -.5, -2, 1, .045, 4);
      mesh(box, silver, 0, -.48, 1, 1, .01, 1);
    }
    scene.add(group); return group;
  }
  return {
    layout(next) {
      for (const [key, group] of zones) if (!next.has(key)) { scene.remove(group); zones.delete(key); }
      for (const [key, zone] of next) {
        if (!zones.has(key)) zones.set(key, make(zone.type || (zone.battlefield ? 'battlefield' : 'hand')));
        const group = zones.get(key), r = zone.rect;
        group.position.set((r.left + r.right) / 2, -(r.top + r.bottom) / 2, -22);
        group.scale.set(r.right - r.left, r.bottom - r.top, 1);
      }
    },
    draw(time) { rift.uniforms.time.value = time; },
  };
}
