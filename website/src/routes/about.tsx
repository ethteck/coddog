import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/about')({
  component: AboutPage,
});

function AboutPage() {
  return (
    <>
      <section className="about-section">
        <div className="about">
          <h2>History of the 'dog</h2>
          <p>
            When decompiling binaries, you often encounter similar or identical
            functions across different projects. However, identifying these has
            previously required{' '}
            <a href="https://gist.github.com/simonlindholm/1933df6a118910822b9b80f3a3e70494">
              specialized tools
            </a>{' '}
            or manual efforts.
          </p>
          <p>
            In 2020, while working on my first decompilation project, I wrote a
            tool to help us find duplicate functions in the game. This tool,{' '}
            <a href="https://github.com/zeldaret/oot/blob/08bf25fca77dfcbc6a0f96104093c770f445ae49/tools/assist.py">
              assist.py
            </a>
            , was a very straightforward n^2 search script that compared every
            function against every other function.
          </p>
          <p>
            Shortly after, in another project, it was updated to use a{' '}
            <a href="https://github.com/pmret/papermario/blob/ee7f09bb2b51a541ac5a12c1996b814ba979ae8d/tools/assist.py#L49-L50">
              much fuzzier but more resilient
            </a>{' '}
            approach to matching: all instructions were shifted right by 2
            bytes. This was done to shoddily mask out instruction arguments,
            which allowed us to match functions that were effectively the same
            but had different constants, for example.
          </p>
          <p>
            One can imagine the difficulty of decompiling all functions of a
            program on a bell curve. There are several that are trivial to
            decompile, a large amount that are non-trivial but not too
            difficult, and some portion that are quite difficult. As the number
            of remaining unmatched functions nears 0, remainder inevitably
            contain a higher proportion of difficult cases. Additionally, these
            functions are typically larger in size and far less likely to have
            full duplicates. So what can be done to help decompile these?
          </p>
          <p>
            In 2022, after talking with some friends in the decomp space, I
            wanted to try a new approach: Rather than searching for entire
            function matches, can we match <em>portions</em> of functions
            against each other? These aforementioned difficult functions
            typically only had issues in localized places, so if we could find
            matches for these patterns of instructions, we could find the
            corresponding C code that generated these matches. By 'patching' the
            non-matching function with the snippet of code from the matching
            one, we could theoretically solve our non-matching issue!
          </p>
          <p>
            I prototyped the idea in Python, and it proved to actually be
            effective. I found use for it in a couple other decomps, and soon,
            others adopted the script for their own projects. However, I wanted
            to be able to search more data. coddog could be a centralized
            database that's publicly searchable, rather than just a local CLI
            tool. In 2024, I ported the core functionality to Rust. In 2025, I
            adapted the matching system to work in a SQL database and began
            creating an API and website.
          </p>
        </div>
      </section>

      <section className="about-section">
        <div className="about">
          <h2>What is and isn't coddog?</h2>
          <p>
            Several solutions for comparing binaries exist today. coddog's
            strength is that it is catered to assisting matching decompilation
            projects. The underlying algorithm works by comparing sequences of
            instructions. While this works great for some cases, it is a strict
            metric and does not work for others.
          </p>
          <p>
            Some examples of things coddog cannot handle include structural
            changes to functions, changes to compiler settings that cause
            codegen differences, or changes to a different architecture
            altogether. More adaptive similarity tools such as{' '}
            <a href="https://github.com/NationalSecurityAgency/ghidra/blob/master/GhidraDocs/GhidraClass/BSim/BSimTutorial_Intro.md">
              Ghidra's BSim
            </a>{' '}
            may be preferable in these cases.
          </p>
          <p>
            However, coddog can excel at finding matches for different versions
            of functions that have minor changes or none at all, such as
            different relocations or different instruction arguments. It can
            also help you find duplicate functions within the same binary, which
            can enable you to create an 'include' file in your project with the
            decompiled function and then reference it in multiple places,
            reducing code duplication and improving maintainability.
          </p>
          <p>
            Of course, coddog also does sub-function matches, which can be
            extremely helpful in resolving issues when matching larger
            functions.
          </p>
        </div>
      </section>
    </>
  );
}
