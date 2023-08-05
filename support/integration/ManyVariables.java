public class ManyVariables {
  public static void main(String[] args) {
    int a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t;
    a = b = c = d = e = f = g = h = i = j = k = l = m = n = o = p = q = r = s = t = 0;
    for (int index = 0; index < 5000; index++) {
      a += index;
      b += index;
      c += index;
      d += index;
      e += index;
      f += index;
      g += index;
      h += index;
      i += index;
      j += index;
      k += index;
      l += index;
      m += index;
      n += index;
      o += index;
      p += index;
      q += index;
      r += index;
      s += index;
      t += index;
    }
    System.out.println(a + b + c + d + e + f + g);
    System.out.println(h + i + j + k + l + m + n);
    System.out.println(o + p + q + r + s + t);
  }
}
